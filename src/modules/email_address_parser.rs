use core::fmt;
use email_address_parser::EmailAddress;
use serde::{Deserializer, Serialize, Serializer};
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct EmailAddressWrapped(pub EmailAddress);

impl fmt::Display for EmailAddressWrapped {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.0.to_string())
  }
}

impl From<String> for EmailAddressWrapped {
  fn from(x: String) -> Self {
    Self(EmailAddress::parse(&x, None).expect("EmailAddress failed to parse."))
  }
}

impl From<&str> for EmailAddressWrapped {
  fn from(x: &str) -> Self {
    Self(EmailAddress::parse(x, None).expect("EmailAddress failed to parse."))
  }
}

impl From<EmailAddress> for EmailAddressWrapped {
  fn from(x: EmailAddress) -> Self {
    Self(x)
  }
}

impl Serialize for EmailAddressWrapped {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.serialize_str(&self.to_string())
  }
}

impl<'de> serde::Deserialize<'de> for EmailAddressWrapped {
  fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
    Ok(Self(
      EmailAddress::parse(&String::deserialize(d)?, None).expect("EmailAddress failed to parse."),
    ))
  }
}
