use anyhow::Result;
use cowstr::CowStr;
use once_cell::sync::Lazy;
use path_clean::clean;
use std::{
  borrow::Cow,
  fmt::Debug,
  fs::canonicalize,
  path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PathSanitizeError {
  #[error("Invalid path: {0}")]
  InvalidPath(CowStr),

  #[error("Path resolution error: {0}")]
  PathResolutionError(CowStr),
}

pub struct API;

static DIR_HOME_PATH_BUF: Lazy<PathBuf> =
  Lazy::new(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp")));
static DIR_HOME_STR: Lazy<CowStr> = Lazy::new(|| {
  DIR_HOME_PATH_BUF
    .clone()
    .into_os_string()
    .into_string()
    .unwrap_or_else(|_| "/tmp".to_owned())
    .into()
});

impl API {
  fn sanitize_internal<'a>(
    input: impl AsRef<str> + Debug,
  ) -> Result<Cow<'a, Path>, PathSanitizeError> {
    let sanitized_input = input
      .as_ref()
      .replace("~/", &format!("{}/", &DIR_HOME_STR.as_str()));

    Ok(Cow::from(clean(&sanitized_input)))
  }

  pub fn sanitize_path<'a>(
    path_input: impl AsRef<Path> + Debug,
  ) -> Result<Cow<'a, Path>, PathSanitizeError> {
    Self::sanitize_internal(path_input.as_ref().to_string_lossy())
  }

  pub fn sanitize_path_resolved<'a>(
    path_input: impl AsRef<Path> + Debug,
  ) -> Result<Cow<'a, Path>, PathSanitizeError> {
    let sanitized_path = Self::sanitize_internal(path_input.as_ref().to_string_lossy())
      .map_err(|e| PathSanitizeError::InvalidPath(CowStr::from(e.to_string())))?;

    let resolved_path = canonicalize(sanitized_path.as_ref())
      .map_err(|e| PathSanitizeError::PathResolutionError(CowStr::from(e.to_string())))?;

    Ok(Cow::from(resolved_path))
  }
}

pub trait PathSanitizeExt {
  fn sanitize_path(&self) -> Result<Cow<Path>, PathSanitizeError>;
}

impl PathSanitizeExt for str {
  fn sanitize_path(&self) -> Result<Cow<Path>, PathSanitizeError> {
    API::sanitize_path(self)
  }
}

impl PathSanitizeExt for Path {
  fn sanitize_path(&self) -> Result<Cow<Path>, PathSanitizeError> {
    API::sanitize_path(self)
  }
}

pub trait PathResolvedSanitizeExt {
  fn sanitize_path_resolved<'a>(&self) -> Result<Cow<'a, Path>, PathSanitizeError>;
}

impl PathResolvedSanitizeExt for str {
  fn sanitize_path_resolved<'a>(&self) -> Result<Cow<'a, Path>, PathSanitizeError> {
    API::sanitize_path_resolved(self)
  }
}

impl PathResolvedSanitizeExt for Path {
  fn sanitize_path_resolved<'a>(&self) -> Result<Cow<'a, Path>, PathSanitizeError> {
    API::sanitize_path_resolved(self)
  }
}
