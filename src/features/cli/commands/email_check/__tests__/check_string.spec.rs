use anyhow::Result;

use super::Command;

#[cfg(feature = "tests_integration")]
#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
async fn cli_command_check_string() -> Result<()> {
  use crate::features::cli::command_common::CommandCheckStringOptions;

  Command::execute(&CommandCheckStringOptions {
    input: "a@b.c;ldddad@tja.coi;dsada@gmail.com;ojs@l.o".into(),
    concurrency: 100,
    timeout_seconds: 10,
    names_email_role_based: None,
  })
  .await
  .unwrap();

  Ok(())
}
