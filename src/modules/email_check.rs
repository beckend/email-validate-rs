use anyhow::{Context, Result};
use check_if_email_exists::{check_email, CheckEmailInput, CheckEmailOutput, Reachable};
use cowstr::CowStr;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_variant::to_variant_name;

// this one is bad: https://github.com/reacherhq/check-if-email-exists/issues/940
const EMAIL_CONTACT_ROLE_BASED: &[&str] = &[
  "support",
  "info",
  "hello",
  "finance",
  "contact",
  "sales",
  "accounting",
  "help",
  "admin",
];

static EMAIL_CONTACT_ROLE_BASED_VEC: Lazy<Vec<CowStr>> = Lazy::new(|| {
  EMAIL_CONTACT_ROLE_BASED
    .iter()
    .map(|x| format!("{}@", x).into())
    .collect()
});

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OptionsIsValid {
  pub names_email_role_based: Option<Vec<CowStr>>,
}

pub struct EmailCheck {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EmailCheckIsValid {
  pub address_email: CowStr,
  pub is_valid: bool,
  pub reasons_failure: Vec<CowStr>,
}

impl EmailCheck {
  pub async fn check_single<TInput>(
    x: TInput,
    options: &OptionsIsValid,
  ) -> Result<(EmailCheckIsValid, CheckEmailOutput)>
  where
    TInput: AsRef<str> + Into<String>,
  {
    let result = Self::fetch_state_single(x).await?;
    Ok((Self::is_valid(&result, options)?, result))
  }

  pub fn is_valid(input: &CheckEmailOutput, options: &OptionsIsValid) -> Result<EmailCheckIsValid> {
    let address_email: CowStr = input.input.clone().into();
    let mut reasons_failure: Vec<CowStr> = Vec::new();

    #[allow(clippy::collapsible_if)]
    if input.is_reachable != Reachable::Safe {
      if input.is_reachable == Reachable::Risky
        && 
        // when it's not a whitelisted role based email
        !options
          .names_email_role_based.clone()
          .unwrap_or_else(|| EMAIL_CONTACT_ROLE_BASED_VEC.clone())
          .iter()
          .any(|x| address_email.starts_with(x.as_str()))
      {
        reasons_failure.push(
          format!(
            "is_reachable: {}",
            to_variant_name(&input.is_reachable).context("is_reachable to_variant_name")?
          )
          .into(),
        );
      }
    }

    if let Ok(misc) = &input.misc {
      if misc.is_disposable {
        reasons_failure.push("is_disposable: true".into());
      }
    }

    if let Ok(smtp) = &input.smtp {
      if !smtp.can_connect_smtp {
        reasons_failure.push("can_connect_smtp: false".into());
      }

      if smtp.has_full_inbox {
        reasons_failure.push("has_full_inbox: true".into());
      }

      if smtp.is_catch_all {
        reasons_failure.push("is_catch_all: true".into());
      }

      if !smtp.is_deliverable {
        reasons_failure.push("is_deliverable: false".into());
      }

      if smtp.is_disabled {
        reasons_failure.push("is_disabled: true".into());
      }
    }

    if !input.syntax.is_valid_syntax {
      reasons_failure.push("is_valid_syntax: false".into());
    }

    Ok(EmailCheckIsValid {
      address_email,
      is_valid: reasons_failure.is_empty(),
      reasons_failure,
    })
  }

  pub async fn fetch_state_single<TInput>(x: TInput) -> Result<CheckEmailOutput>
  where
    TInput: AsRef<str> + Into<String>,
  {
    Ok(check_email(&CheckEmailInput::new(x.into())).await)
  }
}
