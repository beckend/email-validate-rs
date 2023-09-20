use std::num::ParseIntError;
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum ValidationError {
  #[error("Value must be at least 1")]
  ValueBelow1,
  #[error("Error parsing number {0}")]
  ParseError(#[from] ParseIntError),
}

pub struct Validators {}

impl Validators {
  pub fn validate_duration_above_0(value: &u64) -> Result<(), ValidationError> {
    if *value >= 1 {
      Ok(())
    } else {
      Err(ValidationError::ValueBelow1)
    }
  }
}
