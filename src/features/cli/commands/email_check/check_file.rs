use anyhow::Result;
use std::path::Path;

use super::common::Api;
use crate::features::cli::command_common::{CommandOptionsCheckDir, CommandOptionsCheckFile};

#[derive(Debug, Clone)]
pub struct Command {}

impl Command {
  pub async fn execute(options: &CommandOptionsCheckFile) -> Result<()> {
    let options_dir = CommandOptionsCheckDir {
      dir_input: std::borrow::Cow::from(Path::new("/")),
      dir_output: options.dir_output.clone(),
      concurrency: options.concurrency,
      timeout_seconds: options.timeout_seconds,
    };

    let files_paths = options
      .clone()
      .file_input
      .into_iter()
      .map(|x| x.into_owned())
      .collect();

    Api::do_handle_directory_recursive(&options_dir, files_paths, None).await
  }
}

#[cfg(test)]
#[path = "./__tests__/check_file.spec.rs"]
mod features_cli_commands_email_check_check_file;
