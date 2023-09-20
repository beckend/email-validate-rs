use super::{
  command_common::{CLISubCommands, CommandMainCLI},
  commands,
};
use anyhow::Result;
use clap::Parser;

pub struct CommandMain {}

impl CommandMain {
  pub async fn execute() -> Result<()> {
    let command_main_cli = CommandMainCLI::parse();

    match &command_main_cli.command_sub {
      CLISubCommands::CheckDir(x) => commands::email_check::check_dir::Command::execute(x).await,
      CLISubCommands::CheckFile(x) => commands::email_check::check_file::Command::execute(x).await,
      CLISubCommands::CheckString(x) => {
        commands::email_check::check_string::Command::execute(x).await
      }
      CLISubCommands::Experiments(_) => {
        commands::experiments::Command::execute(&command_main_cli).await
      }
    }
  }
}
