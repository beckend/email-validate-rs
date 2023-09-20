use std::fmt::Debug;
use anyhow::Result;

use crate::features::cli::command_common::CommandMainCLI;

#[derive(Debug, Clone)]
pub struct Command {}

impl Command {
  pub async fn execute(_: &CommandMainCLI) -> Result<()> {
    Ok(())
  }
}