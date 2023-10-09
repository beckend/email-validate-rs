use anyhow::{Context, Result};
use cowstr::CowStr;
use dashmap::DashMap;
use email_address_parser::EmailAddress;
use futures::StreamExt;
use kanal::AsyncSender;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;
use std::{
  borrow::Cow,
  collections::HashMap,
  fmt::Debug,
  path::{Path, PathBuf},
  sync::Arc,
  time::Duration,
};
use tokio::{
  fs::OpenOptions,
  io::{AsyncReadExt, AsyncWriteExt, BufReader},
  sync::RwLock,
};
use walkdir::{DirEntry, WalkDir};

use crate::{
  features::cli::{
    command_common::CommandOptionsCheckDir, commands::email_check::tui::PayloadTUIUpdateTotal,
  },
  model::INFALLIBLE,
  modules::{
    email_address_parser::EmailAddressWrapped,
    email_check::{EmailCheck, EmailCheckIsValid},
    fs::write::FileWriter,
    logger::Logger,
  },
};

use super::tui::{PayloadTUIUpdate, TUIChannelPayload, TUIUpdateDispatch, Tui};

static LOG: Lazy<slog::Logger> =
  Lazy::new(|| slog::Logger::root(Logger::new().unwrap().drain, slog::o!("command" => "check")));

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct ItemTimeout {
  pub address_email: CowStr,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct SingleFile<'a> {
  pub path: Cow<'a, Path>,
  pub items: Option<Vec<(EmailAddressWrapped, CowStr)>>,
  pub items_duplicates: Option<Vec<CowStr>>,
  pub count_total: usize,
  pub valids: Vec<EmailCheckIsValid>,
  pub invalids: Vec<EmailCheckIsValid>,
  pub timeouts: Vec<ItemTimeout>,
  pub timings: Vec<(Duration, EmailCheckIsValid)>,
}

#[derive(Debug, Clone)]
pub struct Api {}

impl Api {
  async fn checks(x: &CommandOptionsCheckDir) -> Result<()> {
    // Create output directory to check if we can output anything
    tokio::fs::create_dir_all(&x.dir_output).await?;

    Ok(())
  }

  fn get_files_to_process<TInput: AsRef<Path>>(x: TInput) -> Result<Vec<DirEntry>> {
    let mut returned = Vec::new();

    for entry in WalkDir::new(x).follow_links(true) {
      let entry = entry.context("get entry")?;

      let entry_type = entry.file_type();

      if !entry_type.is_file() || !entry.file_name().to_string_lossy().ends_with(".csv") {
        continue;
      }

      returned.push(entry);
    }

    Ok(returned)
  }

  fn dashmap_arc_to_vec<K, V>(input: &Arc<DashMap<K, V>>) -> DashMap<K, V>
  where
    K: Eq + std::hash::Hash + Clone,
    V: Clone,
  {
    DashMap::from_iter(
      input
        .iter()
        .map(|x| (x.key().clone(), x.value().clone()))
        .collect::<Vec<_>>(),
    )
  }

  // return: first are item, second are duplicates
  pub fn get_csv_items<TInput: AsRef<str>>(
    x: TInput,
    additional_senders_per_domain: Option<&Vec<CowStr>>,
  ) -> (Vec<(EmailAddressWrapped, CowStr)>, Vec<CowStr>, usize) {
    let mut map_exists = HashMap::<CowStr, bool>::new();

    let mut returned = x.as_ref().split([' ', ',', ';', '\n'].as_ref()).fold(
      (
        Vec::<(EmailAddressWrapped, CowStr)>::new(),
        Vec::<CowStr>::new(),
        0,
      ),
      |mut acc, x| {
        let val = x.trim();

        if val.is_empty() {
          return acc;
        }

        let Some(email) = EmailAddress::parse(val, None) else {
          return acc;
        };
        if map_exists.contains_key(val) {
          acc.1.push(val.into());
          return acc;
        }
        let v: CowStr = val.into();
        map_exists.insert(v.clone(), true);
        acc.0.push((email.into(), v));

        acc
      },
    );

    if let Some(adds) = additional_senders_per_domain {
      // need 2 operations, adding to the current iter is bad
      let mut add_to_returned = returned.0.iter().fold(
        (adds, Vec::<(EmailAddressWrapped, CowStr)>::new()),
        |mut acc, unique| {
          for sender in acc.0.iter() {
            let wanted_email: CowStr = format!("{sender}@{}", unique.0 .0.get_domain()).into();

            if !map_exists.contains_key(&wanted_email) {
              acc.1.push((wanted_email.as_str().into(), wanted_email));
              returned.2 += 1;
            }
          }

          acc
        },
      );

      returned.0.append(&mut add_to_returned.1);
    }

    returned
  }

  async fn process_single_item(
    email_address: (EmailAddressWrapped, CowStr),
    state: Arc<RwLock<Option<SingleFile<'_>>>>,
    timeouts_per_domain: Option<Arc<DashMap<CowStr, bool>>>,
    sender_update: Option<AsyncSender<TUIChannelPayload>>,
    timeout_seconds: u64,
  ) -> Result<()> {
    async fn report_err_timeout(
      email_address: &(EmailAddressWrapped, CowStr),
      state: &Arc<RwLock<Option<SingleFile<'_>>>>,
      sender_update: &Option<AsyncSender<TUIChannelPayload>>,
      timeout_seconds: u64,
    ) {
      {
        let mut lock = state.write().await;
        let state = lock.as_mut().expect("singlefile");
        state.count_total += 1;
        state.timeouts.push(ItemTimeout {
          address_email: email_address.1.clone(),
        });
      }

      let time_ms_timeout = Duration::from_secs(timeout_seconds);
      if let Some(tx) = sender_update {
        tx.send(TUIUpdateDispatch::Update(PayloadTUIUpdate {
          email: (email_address.1.clone(), time_ms_timeout, false),
          count_invalid: 0,
          count_valid: 0,
          count_timeout: 1,
          count_total: 1,
        }))
        .await
        .expect("sending update due to timeout");
      }
    }

    if let Some(timeouts_map) = timeouts_per_domain.clone() {
      if timeouts_map.contains_key(email_address.0 .0.get_domain()) {
        report_err_timeout(&email_address, &state, &sender_update, timeout_seconds).await;
        return Ok(());
      }
    }

    let fut = {
      let state = state.clone();
      let email_address = email_address.clone();
      let sender_update_c = sender_update.clone();

      async move {
        let sender_update = sender_update_c;
        let t_start = minstant::Instant::now();
        let t: &str = email_address.1.as_ref();
        let (result, _) = EmailCheck::check_single(t).await?;

        let mut lock = state.write().await;
        let state = lock.as_mut().expect("singlefile");
        let is_valid = result.is_valid;
        let time_duration = t_start.elapsed();

        {
          if is_valid {
            state.valids.push(result.clone());
          } else {
            state.invalids.push(result.clone());
          }
          state.count_total += 1;
          state.timings.push((time_duration, result));
        }
        drop(lock);

        if let Some(tx) = sender_update {
          tx.send(TUIUpdateDispatch::Update(PayloadTUIUpdate {
            email: (email_address.1, t_start.elapsed(), is_valid),
            count_total: 1,
            count_invalid: if is_valid { 0 } else { 1 },
            count_valid: if is_valid { 1 } else { 0 },
            count_timeout: 0,
          }))
          .await
          .expect("sending update from check email results");
        }

        Ok::<(), anyhow::Error>(())
      }
    };

    let time_ms_timeout = Duration::from_secs(timeout_seconds);

    if tokio::time::timeout(time_ms_timeout, fut).await.is_err() {
      if let Some(timeouts_map) = timeouts_per_domain {
        timeouts_map.insert(email_address.0 .0.get_domain().into(), true);
      }
      report_err_timeout(&email_address, &state, &sender_update, timeout_seconds).await;
    }

    Ok(())
  }

  pub async fn process_batch(
    state: Arc<RwLock<Option<SingleFile<'static>>>>,
    sender_update: Option<AsyncSender<TUIChannelPayload>>,
    timeouts_per_domain: Option<Arc<DashMap<CowStr, bool>>>,
    timeout_seconds: u64,
    concurrency: usize,
  ) -> Result<()> {
    let mut s_lock = state.write().await;
    if s_lock.as_ref().expect("singlefile").items.is_none() {
      return Ok(());
    }

    let state_temp = s_lock.as_mut().expect("singlefile");
    let mut items = state_temp.items.take().unwrap();
    items.sort_by(|a, b| a.0 .0.get_domain().cmp(b.0 .0.get_domain()));

    // so the domains are shuffled, the timeouts can be spread across, so when a timeout occurs, the later item will not try to contact that domain
    // if no shuffle, then all items from the same domain are tried in parallell and it's multiple of the same domain times outs which is pointless
    use rand::seq::SliceRandom;
    use rand::thread_rng;
    items.shuffle(&mut thread_rng());

    drop(s_lock);

    let tasks = futures::stream::iter(items.into_iter())
      .map(|x| {
        let state = state.clone();
        let sender_update = sender_update.clone();
        let timeouts_per_domain = timeouts_per_domain.clone();

        tokio::spawn({
          async move {
            Self::process_single_item(
              x,
              state,
              timeouts_per_domain,
              sender_update,
              timeout_seconds,
            )
            .await
          }
        })
      })
      .buffer_unordered(concurrency)
      .collect::<Vec<_>>();

    for x in tasks.await {
      x??;
    }

    Ok(())
  }

  async fn process_single_file_write_output<TPathFile: AsRef<Path>>(
    path_file: TPathFile,
    options: &CommandOptionsCheckDir,
    state: Arc<RwLock<Option<SingleFile<'static>>>>,
    timeouts_per_domain: Option<Arc<DashMap<CowStr, bool>>>,
    sender_update: Option<AsyncSender<TUIChannelPayload>>,
  ) -> Result<()> {
    Self::process_batch(
      state.clone(),
      sender_update.clone(),
      timeouts_per_domain,
      options.timeout_seconds,
      options.concurrency,
    )
    .await?;
    Self::write_output_files(sender_update, options, state, path_file).await?;

    Ok(())
  }

  async fn get_single_file_state<'a>(
    sender_update: Option<AsyncSender<TUIChannelPayload>>,
    options: &CommandOptionsCheckDir,
    path_file: PathBuf,
  ) -> Result<SingleFile<'a>> {
    let mut file = BufReader::new(
      OpenOptions::new()
        .create(false)
        .write(false)
        .read(true)
        .open(&path_file)
        .await?,
    );

    let mut path_file_no_base = path_file
      .clone()
      .as_os_str()
      .to_string_lossy()
      .replace(options.dir_input.as_os_str().to_string_lossy().as_ref(), "");

    // remove the first slash
    path_file_no_base.remove(0);

    let mut content = String::new();
    file.read_to_string(&mut content).await?;

    if let Some(tx) = sender_update.clone() {
      tx.send(TUIUpdateDispatch::MessageMain(
        format!("Path: {}\n", path_file.display()).into(),
      ))
      .await
      .expect("send message main file path");
    }

    let (items, items_duplicates, items_added) =
      Self::get_csv_items(&content, options.additional_senders_per_domain.as_ref());

    if items_added != 0 {
      if let Some(tx) = sender_update {
        tx.send(TUIUpdateDispatch::MessageMain(
          format!(
            "Emails added for every domain: {} - {}\n",
            items_added,
            path_file.file_name().expect("file_name").to_string_lossy(),
          )
          .into(),
        ))
        .await
        .expect("send message main items_added");
      }
    }

    Ok(SingleFile {
      path: Cow::from(path_file),
      items: Some(items),
      items_duplicates: Some(items_duplicates),
      ..Default::default()
    })
  }

  async fn write_output_files<TPathFile: AsRef<Path>>(
    sender_update: Option<AsyncSender<TUIChannelPayload>>,
    options: &CommandOptionsCheckDir,
    state: Arc<RwLock<Option<SingleFile<'_>>>>,
    path_file: TPathFile,
  ) -> Result<()> {
    let mut path_file_no_base: CowStr = path_file
      .as_ref()
      .as_os_str()
      .to_string_lossy()
      .replace(options.dir_input.as_os_str().to_string_lossy().as_ref(), "")
      .into();

    // remove the first slash
    path_file_no_base.remove(0);

    let path_file_no_base_ref: &str = path_file_no_base.as_ref();

    let path_file_output = options
      .dir_output
      .join(path_file_no_base_ref)
      .to_str()
      .unwrap()
      .to_owned();

    let path_file_output = Path::new(&path_file_output);
    let path_file_output_extension = path_file_output.extension().unwrap().to_string_lossy();

    let lock = state.read().await;
    let state = lock.as_ref().expect("singlestate");
    let mut state_c = state.clone();
    let has_timings = !state.timings.is_empty();
    let has_timeouts = !state.timeouts.is_empty();
    drop(lock);

    let path_file_no_extension =
      path_file_no_base_ref.replace(&format!(".{}", path_file_output_extension), "");

    let mut tasks = [
      (
        "Valids",
        state_c.valids,
        options.dir_output.join(format!(
          "{}-valid.{}",
          path_file_no_extension, path_file_output_extension
        )),
      ),
      (
        "Invalids",
        state_c.invalids,
        options.dir_output.join(format!(
          "{}-invalid.{}",
          path_file_no_extension, path_file_output_extension
        )),
      ),
    ]
    .into_iter()
    .filter(|(_, items, _)| !items.is_empty())
    .map(|(label, items, path_write)| {
      let path_write = path_write.clone();
      let sender_update = sender_update.clone();

      tokio::spawn(async move {
        if items.is_empty() {
          tokio::fs::remove_file(path_write).await.ok();
          return Ok::<(), anyhow::Error>(());
        }

        let mut buf_writer = Self::get_file_writer(&path_write).await?;

        Self::write_bytes_to_file(&mut buf_writer, "email\n".as_bytes()).await?;

        for x in items.into_iter() {
          Self::write_bytes_to_file(&mut buf_writer, format!("{}\n", x.address_email).as_bytes())
            .await?;
        }

        buf_writer.flush().await?;

        if let Some(tx) = sender_update {
          tx.send(TUIUpdateDispatch::MessageMain(
            format!(
              "{} written to: {}\n",
              label,
              path_write.canonicalize().expect("canonicalize").display(),
            )
            .into(),
          ))
          .await
          .unwrap_or_else(|x| panic!("{label} send message main written count. {}", x));
        }

        Ok::<(), anyhow::Error>(())
      })
    })
    .collect::<Vec<_>>();

    if has_timings {
      tasks.push(tokio::spawn({
        let path_write = options
          .dir_output
          .join(format!("{}-timing.json", path_file_no_extension));

        if state_c.timings.is_empty() {
          tokio::fs::remove_file(path_write).await.ok();
          return Ok::<(), anyhow::Error>(());
        }

        let sender_update = sender_update.clone();

        async move {
          #[derive(Serialize)]
          struct Item {
            pub data: EmailCheckIsValid,
            pub time_process: Duration,
          }

          state_c.timings.sort_by(|a, b| b.0.cmp(&a.0));

          let items = state_c
            .timings
            .into_iter()
            .map(|x| Item {
              data: x.1.clone(),
              time_process: x.0,
            })
            .collect::<Vec<_>>();

          FileWriter::builder()
            .filename(Cow::Borrowed(&path_write))
            .build()
            .write(to_string_pretty(&items)?)
            .await?;

          if let Some(tx) = sender_update {
            tx.send(TUIUpdateDispatch::MessageMain(
              format!(
                "Timings report file written to: {}\n",
                path_write.canonicalize().expect("canonicalize").display(),
              )
              .into(),
            ))
            .await
            .expect("send message main timings report written.");
          }

          Ok(())
        }
      }));
    }

    if has_timeouts {
      tasks.push(tokio::spawn({
        let path_write = options.dir_output.join(format!(
          "{}-timeout.{}",
          path_file_no_extension, path_file_output_extension
        ));

        if state_c.timeouts.is_empty() {
          tokio::fs::remove_file(path_write).await.ok();
          return Ok::<(), anyhow::Error>(());
        }

        let sender_update = sender_update.clone();

        async move {
          state_c
            .timeouts
            .sort_by(|a, b| b.address_email.cmp(&a.address_email));

          let mut buf_writer = Self::get_file_writer(&path_write).await?;
          Self::write_bytes_to_file(&mut buf_writer, "email\n".as_bytes()).await?;

          for x in state_c.timeouts.into_iter() {
            Self::write_bytes_to_file(&mut buf_writer, format!("{}\n", x.address_email).as_bytes())
              .await?;
          }

          buf_writer.flush().await?;

          if let Some(tx) = sender_update {
            tx.send(TUIUpdateDispatch::MessageMain(
              format!(
                "Timeouts report file written to: {}\n",
                path_write.canonicalize().expect("canonicalize").display(),
              )
              .into(),
            ))
            .await
            .expect("send message main timeouts report.");
          }

          Ok(())
        }
      }));
    }

    for x in tasks {
      x.await??;
    }

    if let Some(tx) = sender_update {
      tx.send(TUIUpdateDispatch::TotalFilesWritten(
        super::tui::PayloadTUIUpdateFilesWritten { count_total: 1 },
      ))
      .await
      .expect("send update report files written.")
    }

    Ok(())
  }

  async fn get_file_writer<TPathFile: AsRef<Path>>(
    path_file: TPathFile,
  ) -> Result<tokio::io::BufWriter<tokio::fs::File>> {
    use tokio::{
      fs,
      io::{self},
    };

    let path_file = path_file.as_ref();

    if let Some(parent) = path_file.parent() {
      fs::create_dir_all(parent).await?;
    } else {
      return Err(anyhow::anyhow!("failed to get parent directory."));
    }

    let file = OpenOptions::new()
      .write(true)
      .create(true)
      .truncate(true)
      .open(&path_file)
      .await?;
    Ok(io::BufWriter::new(file))
  }

  async fn write_bytes_to_file<TData: AsRef<[u8]>>(
    file: &mut tokio::io::BufWriter<tokio::fs::File>,
    data: TData,
  ) -> Result<usize> {
    file.write(data.as_ref()).await.map_err(anyhow::Error::new)
  }

  pub async fn do_handle_directory_recursive(
    options: &CommandOptionsCheckDir,
    files_paths: Vec<PathBuf>,
    time_fn: Option<coarsetime::Instant>,
  ) -> Result<()> {
    let time_fn = time_fn.unwrap_or_else(coarsetime::Instant::now);
    if files_paths.is_empty() {
      return Err(anyhow::anyhow!("No .csv files found"));
    }

    // the key is the email the vec contains file paths
    let map_duplicates = Arc::new(DashMap::<CowStr, Vec<CowStr>>::new());
    let (tui_update_tx, tui_update_rx) = Tui::new_channels(options.concurrency);

    let mut tasks = Vec::new();

    let task_tui_update = tokio::spawn({
      let tui_update_rx = tui_update_rx.clone();
      async move { Tui::execute(tui_update_rx).await }
    });

    let task_tui_update_timer = tokio::spawn({
      let tui_update_tx = tui_update_tx.clone();
      let timer = Duration::from_millis(100);

      async move {
        loop {
          if tui_update_tx.is_closed() {
            break;
          } else {
            #[allow(clippy::collapsible_else_if)]
            if tui_update_tx
              .send(TUIUpdateDispatch::TimeElapsed(humantime::format_duration(
                humantime::parse_duration(&format!("{}ns", time_fn.elapsed().as_nanos()))?,
              )))
              .await
              .is_err()
            {
              break;
            }
          }

          tokio::time::sleep(timer).await;
        }

        Ok::<(), anyhow::Error>(())
      }
    });

    let msg_begin = [
      format!("Concurrency: {}\n", options.concurrency),
      format!("Timeout: {}s\n", options.timeout_seconds),
      if options.additional_senders_per_domain.is_some() {
        format!(
          "Additional senders per domain: {}s\n\n",
          options
            .additional_senders_per_domain
            .as_ref()
            .expect(INFALLIBLE)
            .join(", ")
        )
      } else {
        "\n".into()
      },
    ]
    .join("");

    tui_update_tx
      .send(TUIUpdateDispatch::MessageMain(msg_begin.into()))
      .await
      .expect("send message main begin.");

    let file_states_tasks = files_paths
      .into_iter()
      .map(|the_path| {
        tokio::spawn({
          let options = options.clone();
          let map_duplicates = map_duplicates.clone();
          let tui_update_tx = tui_update_tx.clone();
          let path_file_str: CowStr = the_path.to_string_lossy().to_string().into();

          async move {
            let mut state =
              Self::get_single_file_state(Some(tui_update_tx), &options, the_path.clone()).await?;

            if let Some(items) = state.items.as_mut() {
              items.retain(|item| {
                if map_duplicates.contains_key(&item.1) {
                  let mut path_dups = map_duplicates.get_mut(&item.1).expect(INFALLIBLE);

                  if !path_dups.contains(&path_file_str) {
                    path_dups.push(path_file_str.clone());
                  }

                  return false;
                }

                map_duplicates.insert(item.1.clone(), vec![path_file_str.clone()]);

                true
              });
            }

            if let Some(items) = state.items_duplicates.as_ref() {
              items.iter().for_each(|item| {
                if map_duplicates.contains_key(item) {
                } else {
                  // for something in the map to count as a dup, it needs to be happening twice, so we insert it twice
                  // the map contains a vec of filepaths which is a dup, it being there twice means that it had itself dups in it's items
                  for _ in 1..3 {
                    map_duplicates.insert(item.to_string().into(), vec![path_file_str.clone()]);
                  }
                }
              });
            }

            Ok::<(PathBuf, SingleFile<'_>), anyhow::Error>((the_path, state))
          }
        })
      })
      .collect::<Vec<_>>();

    let mut file_states = Vec::new();

    for x in file_states_tasks {
      file_states.push(x.await??);
    }

    {
      let path_write = options.dir_output.join("duplicates.json");
      // let map_duplicates = map_duplicates.write().await.take().unwrap();

      {
        // 2 phases since dashmap has sync primitives that dead locks when a borrow is at hand
        let mut not_dups = Vec::new();

        for x in map_duplicates.iter() {
          if x.value().len() < 2 {
            not_dups.push(x.key().to_owned());
          }
        }

        for x in not_dups {
          map_duplicates.remove(&x);
        }
      }

      if map_duplicates.is_empty() {
        tokio::fs::remove_file(path_write).await.ok();
      } else {
        tasks.push(tokio::spawn({
          let tui_update_tx = tui_update_tx.clone();

          async move {
            #[derive(Debug, Serialize)]
            struct FileOut {
              pub duplicates: dashmap::DashMap<CowStr, Vec<CowStr>>,
            }

            FileWriter::builder()
              .filename(Cow::Borrowed(&path_write))
              .build()
              .write(to_string_pretty(&FileOut {
                duplicates: Api::dashmap_arc_to_vec(&map_duplicates),
              })?)
              .await?;

            tui_update_tx
              .send(TUIUpdateDispatch::MessageMain(
                format!(
                  "Duplicates file written to: {}\n",
                  path_write.canonicalize().expect("canonicalize").display(),
                )
                .into(),
              ))
              .await
              .expect("send file written duplicates.");

            Ok(())
          }
        }));
      }
    }

    let all_valids = Arc::new(RwLock::new(Vec::<EmailCheckIsValid>::new()));
    let timeouts_per_domain = Arc::new(DashMap::<CowStr, bool>::new());

    for (path_file, state) in file_states {
      let tui_update_tx = tui_update_tx.clone();
      let items_len = if state.items.is_some() {
        state.items.as_ref().unwrap().len() as u64
      } else {
        0
      };
      tui_update_tx
        .clone()
        .send(TUIUpdateDispatch::Total(PayloadTUIUpdateTotal {
          count_files_total: 1,
          count_total: items_len as usize,
        }))
        .await
        .expect("send message count_totals.");

      tasks.push(tokio::spawn({
        let state_threads = Arc::new(RwLock::new(Some(state)));
        let tui_update_tx = tui_update_tx.clone();
        let options = options.clone();
        let all_valids = all_valids.clone();
        let timeouts_per_domain = timeouts_per_domain.clone();

        async move {
          Self::process_single_file_write_output(
            path_file,
            &options,
            state_threads.clone(),
            Some(timeouts_per_domain),
            Some(tui_update_tx),
          )
          .await?;

          all_valids.write().await.append(
            &mut state_threads
              .read()
              .await
              .clone()
              .expect("state_threads.read")
              .valids
              .clone(),
          );
          Ok::<(), anyhow::Error>(())
        }
      }));
    }

    for x in tasks {
      x.await??;
    }

    if !task_tui_update_timer.is_finished() {
      task_tui_update_timer.abort();
    }

    let path_write_all_valids = options.dir_output.join("all_valids.csv");

    {
      let mut lock = all_valids.write().await;
      let mut items = lock.drain(..).collect::<Vec<_>>();
      drop(lock);
      items.sort_by(|a, b| {
        a.email
          .0
          .get_domain()
          .cmp(b.email.0.get_domain())
          .then(a.email.0.get_local_part().cmp(b.email.0.get_local_part()))
      });

      let mut buf_writer = Self::get_file_writer(&path_write_all_valids).await?;
      Self::write_bytes_to_file(&mut buf_writer, "email\n".as_bytes()).await?;

      for x in items.into_iter() {
        Self::write_bytes_to_file(&mut buf_writer, format!("{}\n", x.address_email).as_bytes())
          .await?;
      }

      buf_writer.flush().await?;
    }

    slog::info!(
      LOG,
      "End: {}",
      humantime::format_duration(humantime::parse_duration(&format!(
        "{}ms",
        time_fn.elapsed().as_millis()
      ))?);
    );

    println!("{}", task_tui_update.await??.message_main);

    slog::info!(
      LOG,
      "All valids written to: {}\n",
      path_write_all_valids
        .canonicalize()
        .expect("canonicalize")
        .display(),
    );

    Ok(())
  }

  async fn do_handle_directory(options: &CommandOptionsCheckDir) -> Result<()> {
    let time_fn = coarsetime::Instant::now();
    slog::info!(LOG, "Start");
    Self::checks(options).await?;

    let files_paths = Self::get_files_to_process(options.dir_input.as_ref())?;

    Self::do_handle_directory_recursive(
      options,
      files_paths
        .into_iter()
        .map(|x| x.path().canonicalize().expect("canonicalize"))
        .collect(),
      Some(time_fn),
    )
    .await
  }
}

impl Api {
  // so we can be sure std io has been flushed
  pub async fn handle_directory(options: &CommandOptionsCheckDir) -> Result<()> {
    Self::do_handle_directory(options).await?;

    tokio::time::sleep(Duration::from_millis(100)).await;
    // Lingering network connections prevents the app from quitting, however we are done.
    std::process::exit(0);
  }
}
