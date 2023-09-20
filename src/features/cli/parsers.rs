use anyhow::Result;
use std::{
  borrow::Cow,
  path::{Path, PathBuf},
};

use super::validators::{ValidationError, Validators};
pub use crate::modules::fs::path_sanitize::{PathResolvedSanitizeExt, PathSanitizeExt};

pub struct Parsers {}

impl Parsers {
  pub fn path(input: &str) -> Result<Cow<'static, Path>> {
    Ok(Cow::from(PathBuf::from(input.sanitize_path()?)))
  }

  // pub fn to_path_file_resolved(input: &str) -> Result<Cow<'static, Path>> {
  //   Ok(input.sanitize_path_resolved()?)
  // }

  pub fn path_resolved(input: &str) -> Result<Cow<'static, Path>> {
    Ok(input.sanitize_path_resolved()?)
  }

  pub fn parse_duration_above_0(input: &str) -> Result<u64, ValidationError> {
    let v = input.parse::<u64>()?;
    Validators::validate_duration_above_0(&v)?;

    Ok(v)
  }
}
