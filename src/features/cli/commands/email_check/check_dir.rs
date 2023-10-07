use anyhow::Result;

use crate::features::cli::command_common::CommandOptionsCheckDir;

use super::common::Api;

#[derive(Debug, Clone)]
pub struct Command {}

impl Command {
  pub async fn execute(options: &CommandOptionsCheckDir) -> Result<()> {
    Api::handle_directory(options).await
  }
}

#[cfg(test)]
#[path = "./__tests__/check_dir.spec.rs"]
mod features_cli_commands_email_check_check_dir;
