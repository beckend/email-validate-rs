use anyhow::{anyhow, Result};
use colored::{ColoredString, Colorize};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::common::Api;
use crate::{
  features::cli::{
    command_common::CommandOptionsCheckString, commands::email_check::common::SingleFile,
  },
  modules::email_check::EmailCheckIsValid,
};

#[derive(Debug, Clone)]
pub struct Command {}

impl Command {
  pub async fn execute(options: &CommandOptionsCheckString) -> Result<()> {
    let (items, items_dup, items_added) = Api::get_csv_items(
      options.input.as_str(),
      options.additional_senders_per_domain.as_ref(),
    );

    if items.is_empty() {
      return Err(anyhow!("input contained no valid emails."));
    }

    let state = Arc::new(RwLock::new(Some(SingleFile {
      items: Some(items),
      ..Default::default()
    })));

    Api::process_batch(
      state.clone(),
      None,
      None,
      options.timeout_seconds,
      options.concurrency,
    )
    .await?;

    let mut lock = state.write_owned().await;
    let state = lock.take().expect("singlestate");
    let len_duplicates = items_dup.len();
    let len_invalids = state.invalids.len();
    let len_valids = state.valids.len();
    let len_timeouts = state.timeouts.len();

    fn get_list_email<I>(x: I, label: &ColoredString) -> String
    where
      I: Iterator<Item = EmailCheckIsValid>,
    {
      format!(
        "\n{}",
        format_args!(
          "\n{label}: {list}",
          list = x.fold("".into(), |mut acc: String, item| {
            acc += "\n";
            acc += item.address_email.as_ref();

            if !item.reasons_failure.is_empty() {
              acc += item.reasons_failure.join(", ").as_str()
            }

            acc
          })
        ),
      )
    }

    let w = [
      format!("{}: {}\n", "Total".blue(), state.count_total),
      format!("{}: {}\n", "Items added per domain:".blue(), items_added),
      format!("{}: {}\n", "Duplicates".red(), len_duplicates),
      format!("{}: {}\n", "Invalid".red(), len_invalids),
      format!("{}: {}\n", "Timeout".bright_red(), state.timeouts.len()),
      format!("{}: {}", "Valid".green(), len_valids),
      if len_duplicates > 0 {
        format!(
          "\n{}",
          format_args!(
            "\n{label}: {list}",
            label = "Duplicates".bright_magenta(),
            list = items_dup
              .into_iter()
              .fold("".into(), |mut acc: String, item| {
                acc += "\n";
                acc += item.as_ref();

                acc
              })
          ),
        )
      } else {
        "".into()
      },
      if len_invalids > 0 {
        get_list_email(state.invalids.into_iter(), &"Invalid".red())
      } else {
        "".into()
      },
      if len_timeouts > 0 {
        format!(
          "\n{}",
          format_args!(
            "\n{label}: {list}",
            label = "Timeout".bright_red(),
            list = state
              .timeouts
              .into_iter()
              .fold("".into(), |mut acc: String, item| {
                acc += "\n";
                acc += item.address_email.as_ref();

                acc
              })
          ),
        )
      } else {
        "".into()
      },
      if len_valids > 0 {
        get_list_email(state.valids.into_iter(), &"Valid".green())
      } else {
        "".into()
      },
    ]
    .join("");

    use std::io::{self, Write};
    let mut stdout = io::stdout();
    stdout.write_all(w.as_bytes())?;
    stdout.flush()?;

    Ok(())
  }
}

#[cfg(test)]
#[path = "./__tests__/check_string.spec.rs"]
mod features_cli_commands_email_check_check_string;
