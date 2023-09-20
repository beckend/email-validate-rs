use anyhow::Result;
pub use slog::{self, Drain};
pub use slog_async;
use std::sync::Arc;

static ENVIRONMENT_KEY_LOG_LEVEL: &str = "LOG_LEVEL";

#[derive(Clone)]
pub struct Logger {
  pub drain: Arc<slog::Fuse<slog_async::Async>>,
  pub log_level: slog::Level,
}

pub struct CustomFormatter;
impl slog::Drain for CustomFormatter {
  type Ok = ();
  type Err = ();
  fn log(
    &self,
    record: &slog::Record,
    _values: &slog::OwnedKVList,
  ) -> std::result::Result<Self::Ok, Self::Err> {
    println!(
      "{} {} {} {}: {}",
      record.level(),
      record.module(),
      record.file(),
      record.line(),
      record.msg()
    );
    Ok(())
  }
}

impl Logger {
  pub fn new() -> Result<Self> {
    let lvl_log = match std::env::var(ENVIRONMENT_KEY_LOG_LEVEL) {
      Ok(_) => match std::env::var(ENVIRONMENT_KEY_LOG_LEVEL) {
        Ok(x) => x,
        Err(_e) => "none".to_string(),
      },
      Err(_) => {
        std::env::set_var(ENVIRONMENT_KEY_LOG_LEVEL, "info");
        "info".to_string()
      }
    };

    // @TODO figure out why trace does not work
    let log_level = match lvl_log.as_str() {
      "trace" => slog::Level::Trace,
      "debug" => slog::Level::Debug,
      "info" => slog::Level::Info,
      "warning" => slog::Level::Warning,
      "error" => slog::Level::Error,
      "critical" => slog::Level::Critical,
      _ => slog::Level::Critical,
    };

    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator)
      .build()
      .filter_level(log_level)
      .fuse();
    let drain = Arc::new(slog_async::Async::new(drain).build().fuse());

    // let the_log = slog::Logger::root(drain, slog::o!("version" => "0.5"));
    // slog::info!(the_log, "formatted: {}", 1; "log-key" => true);

    Ok(Self { drain, log_level })
  }
}
