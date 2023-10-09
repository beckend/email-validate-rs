use anyhow::Result;

use super::Command;

#[cfg(feature = "tests_integration")]
#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
async fn cli_command_check_dir() -> Result<()> {
  use std::{borrow::Cow, env, path::Path};

  // github does not find any files
  if env::var("CI").unwrap_or("false".into()) == "true" {
    return Ok(());
  }

  use crate::features::cli::command_common::CommandOptionsCheckDir;

  let path_fixtures = Path::new(std::file!()).parent().unwrap().join("fixtures");

  let path_fixtures_target = path_fixtures.canonicalize()?;
  let path_output = tempfile::tempdir()?;
  let path_output = path_output.path().to_owned();

  Command::execute(&CommandOptionsCheckDir {
    additional_senders_per_domain: Some(
      "contact,info,hello"
        .split('"')
        .map(|x| x.into())
        .collect::<Vec<_>>(),
    ),
    dir_input: Cow::from(path_fixtures_target),
    dir_output: Cow::from(path_output),
    concurrency: 100,
    timeout_seconds: 10,
  })
  .await
  .unwrap();

  Ok(())
}
