use anyhow::{Context, Result};
use cowstr::CowStr;
use crossterm::{
  event::{DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers},
  execute,
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use humantime::FormattedDuration;
use kanal::{AsyncReceiver, AsyncSender};
use ratatui::{prelude::*, widgets::*};
use std::{
  collections::VecDeque,
  io::{self},
  time::Duration,
};

const LIMIT_EMAILS: usize = 10_usize;
type Term = Terminal<CrosstermBackend<io::Stdout>>;

pub enum TUIUpdateDispatch {
  TimeElapsed(FormattedDuration),
  Update(PayloadTUIUpdate),
  MessageMain(CowStr),
  Total(PayloadTUIUpdateTotal),
  TotalFilesWritten(PayloadTUIUpdateFilesWritten),
}

#[derive(Debug, Clone)]
pub struct PayloadTUIUpdateTotal {
  pub count_total: usize,
  pub count_files_total: usize,
}

#[derive(Debug, Clone)]
pub struct PayloadTUIUpdateFilesWritten {
  pub count_total: usize,
}

#[derive(Debug, Clone)]
pub struct PayloadTUIUpdate {
  pub email: (CowStr, Duration, bool),
  pub count_total: usize,
  pub count_invalid: usize,
  pub count_valid: usize,
  pub count_timeout: usize,
}

pub type TUIChannelPayload = TUIUpdateDispatch;

#[derive(Debug, Default)]
pub struct Tui {
  pub duration_default_0: Duration,
  pub email_empty: (CowStr, Duration, bool),

  // this is the every email, email_address, time for to process, is valid
  pub emails: VecDeque<(CowStr, Duration, bool)>,

  pub count_files_total: usize,
  pub count_files_total_processed: usize,

  pub count_total: usize,
  pub count_total_processed: usize,

  pub count_invalid: usize,
  pub count_valid: usize,
  pub count_timeout: usize,
  pub message_main: CowStr,
  pub time_elapsed: Option<FormattedDuration>,
}

impl Tui {
  pub fn new() -> Self {
    Self {
      duration_default_0: Duration::from_nanos(0),
      email_empty: ("".into(), Duration::from_nanos(0), false),
      emails: VecDeque::with_capacity(LIMIT_EMAILS),
      ..Default::default()
    }
  }

  pub fn new_channels(
    limit: usize,
  ) -> (
    AsyncSender<TUIChannelPayload>,
    AsyncReceiver<TUIChannelPayload>,
  ) {
    kanal::bounded_async(limit)
  }
}

impl Tui {
  async fn run_app(
    &mut self,
    terminal: &mut Term,
    rx_update: AsyncReceiver<TUIChannelPayload>,
  ) -> Result<()> {
    let (quit_tx, quit_rx) = kanal::bounded_async::<()>(1);

    tokio::spawn(async move {
      let mut events = crossterm::event::EventStream::new();
      async fn quit(tx: &AsyncSender<()>) -> Result<()> {
        tx.send(()).await.context("quit due to key presses")
      }

      while let Some(Ok(evt)) = events.next().await {
        if let crossterm::event::Event::Key(key) = evt {
          if let KeyCode::Esc = key.code {
            quit(&quit_tx).await?;
            break;
          }

          if let KeyCode::Char('q') = key.code {
            quit(&quit_tx).await?;
            break;
          }

          if key.modifiers.eq(&KeyModifiers::CONTROL) {
            if let KeyCode::Char('c') = key.code {
              quit(&quit_tx).await?;
            }
          }
        }
      }

      Ok::<(), anyhow::Error>(())
    });

    fn limit_email<TItem>(x: &mut VecDeque<TItem>) {
      if x.len() == LIMIT_EMAILS {
        x.pop_back();
      }
    }

    loop {
      tokio::select! {
          Ok(action) = rx_update.recv() => {
           match action {
            TUIUpdateDispatch::TimeElapsed(x) => {
              self.time_elapsed = Some(x);
            }

            TUIUpdateDispatch::Update(update) => {
              limit_email(&mut self.emails);
              self.emails.push_front(update.email);
              self.count_timeout += update.count_timeout;
              self.count_invalid += update.count_invalid;
              self.count_valid += update.count_valid;
              self.count_total_processed += update.count_total;
            }

            TUIUpdateDispatch::MessageMain(update) => {
              self.message_main += update;
            }

            TUIUpdateDispatch::Total(update) => {
              self.count_total += update.count_total;
              self.count_files_total += update.count_files_total;
            }

            TUIUpdateDispatch::TotalFilesWritten(update) => {
              self.count_files_total_processed += update.count_total;
            }
          }

           terminal.draw(|f| self.draw_frame(f))?;

           if self.count_total > 0 &&
            self.count_total_processed >= self.count_total &&
            self.count_files_total > 0 &&
            self.count_files_total_processed >= self.count_files_total {
            rx_update.close();
            quit_rx.close();
            break;
           }
          }

          _ = quit_rx.recv() => {
            Self::terminal_restore(terminal)?;
            std::process::abort();
          }
      }
    }

    Ok(())
  }

  fn draw_frame<B: Backend>(&self, f: &mut Frame<B>) {
    let chunks = Layout::default()
      .direction(Direction::Vertical)
      .constraints(
        [
          Constraint::Percentage(10),
          Constraint::Min(5),
          Constraint::Min(5),
          Constraint::Percentage(80),
        ]
        .as_ref(),
      )
      .split(f.size());

    let ratio_done_gauge = self.count_total_processed as f64 / self.count_total as f64;

    f.render_widget(
      Gauge::default()
        .block(
          Block::default()
            .title("Progress")
            .borders(Borders::ALL)
            .title_alignment(Alignment::Center),
        )
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black))
        .percent(if ratio_done_gauge.is_nan() {
          0
        } else if ratio_done_gauge < 1.0 {
          (ratio_done_gauge * 100.0).floor() as u16
        } else {
          (ratio_done_gauge * 100.0) as u16
        }),
      chunks[0],
    );

    f.render_widget(
      Paragraph::new(format!(
        "{time_elapsed}\nfiles: {files}   filesDone: {files_processed}   total: {total}   done: {done}   timeout: {timeout}   invalid: {invalid}   valid: {valid}",
        time_elapsed = self.time_elapsed.clone().unwrap_or_else(|| humantime::format_duration(Duration::new(0, 0))),
        files = self.count_files_total,
        files_processed = self.count_files_total_processed,
        total = self.count_total,
        done = self.count_total_processed,
        timeout = self.count_timeout,
        invalid = self.count_invalid,
        valid = self.count_valid
      ))
      .style(Style::default().add_modifier(Modifier::BOLD))
      .alignment(Alignment::Center)
      .wrap(Wrap { trim: true }),
      chunks[1],
    );

    // @TODO figure out how to render a list inside that area
    let (email_address, email_process_duration, email_is_valid) =
      self.emails.get(0).unwrap_or(&self.email_empty);

    f.render_widget(
      Paragraph::new(format!(
        "Valid: {email_is_valid}  {email_address}  {time}",
        time = humantime::format_duration(*email_process_duration),
      ))
      .style(
        Style::default()
          .add_modifier(Modifier::ITALIC)
          .fg(if *email_is_valid {
            Color::Green
          } else {
            Color::Red
          }),
      )
      .alignment(Alignment::Center)
      .wrap(Wrap { trim: true }),
      chunks[2],
    );

    f.render_widget(
      Paragraph::new(self.message_main.as_str()).wrap(Wrap { trim: true }),
      chunks[3],
    );
  }

  fn terminal_init() -> Result<Term> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).context("terminal")
  }

  fn terminal_restore(terminal: &mut Term) -> Result<()> {
    disable_raw_mode()?;
    execute!(
      terminal.backend_mut(),
      LeaveAlternateScreen,
      DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
  }

  pub async fn execute(rx_update: AsyncReceiver<TUIChannelPayload>) -> Result<Self> {
    let mut terminal = Self::terminal_init()?;

    let mut instance = Self::new();

    instance
      .run_app(&mut terminal, rx_update)
      .await
      .context("TUI::run_app")?;

    Self::terminal_restore(&mut terminal)?;

    Ok(instance)
  }
}
