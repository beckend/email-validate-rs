mod features;
mod model;
mod modules;

use anyhow::Result;
use features::cli::command_main::CommandMain;

#[tokio::main]
async fn main() -> Result<()> {
  CommandMain::execute().await
}
