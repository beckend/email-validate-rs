use clap::{ArgAction, Args, Parser, Subcommand, ValueHint};
use cowstr::CowStr;
use std::path::Path;
use std::{borrow::Cow, num::NonZeroUsize};

pub use crate::features::cli::{parsers::Parsers, validators::Validators};

#[derive(Args, Debug, Clone, Eq, PartialEq)]
pub struct CommandOptionsCheckDir {
  /// Directory containing .cvs files, it's walked recursively, the emails may be separated by newline, space, coma or semicolon
  #[clap(long, value_hint = ValueHint::DirPath, value_parser = Parsers::path_resolved, required = true)]
  pub dir_input: Cow<'static, Path>,
  /// Directory to output {input-file}-timeout.csv {input-file}-invalid.csv {input-file}-valid.csv {input-file}-timing.json
  #[clap(long, value_hint = ValueHint::DirPath, value_parser = Parsers::path, required = true)]
  pub dir_output: Cow<'static, Path>,
  #[clap(long, default_value_t = NonZeroUsize::new(25).unwrap())]
  pub concurrency: NonZeroUsize,
  /// per item
  #[clap(
    long,
    default_value_t = 120_u64,
    value_parser = Parsers::parse_duration_above_0
  )]
  pub timeout_seconds: u64,
  #[clap(long, value_delimiter = ',')]
  pub additional_senders_per_domain: Option<Vec<CowStr>>,
}

#[derive(Args, Debug, Clone, Eq, PartialEq)]
pub struct CommandOptionsCheckFile {
  /// separated by newline, space, coma or semicolon
  #[clap(long, value_hint = ValueHint::FilePath, value_parser = Parsers::path_resolved, required = true)]
  pub file_input: Vec<Cow<'static, Path>>,
  /// Directory to output {input-file}-timeout.csv {input-file}-invalid.csv {input-file}-valid.csv {input-file}-timing.json
  #[clap(long, value_hint = ValueHint::DirPath, value_parser = Parsers::path, required = true)]
  pub dir_output: Cow<'static, Path>,
  #[clap(long, default_value_t = NonZeroUsize::new(25).unwrap())]
  pub concurrency: NonZeroUsize,
  /// per item
  #[clap(
    long,
    default_value_t = 120_u64,
    value_parser = Parsers::parse_duration_above_0
  )]
  pub timeout_seconds: u64,
  #[clap(long, value_delimiter = ',')]
  pub additional_senders_per_domain: Option<Vec<CowStr>>,
}

#[derive(Args, Debug, Clone, Eq, PartialEq)]
pub struct CommandOptionsCheckString {
  /// separated by space, coma or semicolon
  #[clap(long, required = true)]
  pub input: CowStr,
  #[clap(long, default_value_t = NonZeroUsize::new(25).unwrap())]
  pub concurrency: NonZeroUsize,
  /// per item
  #[clap(
    long,
    default_value_t = 120_u64,
    value_parser = Parsers::parse_duration_above_0
  )]
  pub timeout_seconds: u64,
  #[clap(long, value_delimiter = ',')]
  pub additional_senders_per_domain: Option<Vec<CowStr>>,
}

#[derive(Args, Debug, Clone, Eq, PartialEq)]
pub struct CommandEmptyOptions {}

#[derive(Subcommand, Debug, Clone, Eq, PartialEq)]
pub enum CLISubCommands {
  #[clap(name = "check-dir")]
  CheckDir(CommandOptionsCheckDir),
  #[clap(name = "check-file")]
  CheckFile(CommandOptionsCheckFile),
  #[clap(name = "check-string")]
  CheckString(CommandOptionsCheckString),
  #[clap(name = "experiments")]
  Experiments(CommandEmptyOptions),
}

#[derive(Parser, Debug, Clone, Eq, PartialEq)]
#[clap(name = "email_check", author, version, about, long_about = None, propagate_version = true)]
pub struct CommandMainCLI {
  #[clap(long, action)]
  pub debug: bool,

  // The number of occurrences of the `v/verbose` flag
  /// Verbose mode (-v, -vv, -vvv, etc.)
  #[clap(long, action = ArgAction::Count)]
  pub verbose: u8,

  #[clap(subcommand)]
  pub command_sub: CLISubCommands,
}
