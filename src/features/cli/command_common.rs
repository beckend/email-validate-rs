use clap::{ArgAction, Args, Parser, Subcommand, ValueHint};
use cowstr::CowStr;
use std::borrow::Cow;
use std::path::Path;

pub use crate::features::cli::{parsers::Parsers, validators::Validators};

#[derive(Args, Debug, Clone, Eq, PartialEq)]
pub struct CommandCheckDirOptions {
  /// Directory containing .cvs files, it's walked recursively, the emails may be separated by newline, space, coma or semicolon
  #[clap(long, value_hint = ValueHint::DirPath, value_parser = Parsers::path_resolved, required = true)]
  pub dir_input: Cow<'static, Path>,
  /// Directory to output {input-file}-timeout.csv {input-file}-invalid.csv {input-file}-valid.csv {input-file}-timing.json
  #[clap(long, value_hint = ValueHint::DirPath, value_parser = Parsers::path, required = true)]
  pub dir_output: Cow<'static, Path>,
  #[clap(long, default_value_t = 25_usize)]
  pub concurrency: usize,
  /// per item
  #[clap(
    long,
    default_value_t = 120_u64,
    value_parser = Parsers::parse_duration_above_0
  )]
  pub timeout_seconds: u64,
  #[clap(long, value_delimiter = ',')]
  pub names_email_role_based: Option<Vec<CowStr>>,
}

#[derive(Args, Debug, Clone, Eq, PartialEq)]
pub struct CommandCheckFileOptions {
  /// separated by newline, space, coma or semicolon
  #[clap(long, value_hint = ValueHint::FilePath, value_parser = Parsers::path_resolved, required = true)]
  pub file_input: Vec<Cow<'static, Path>>,
  /// Directory to output {input-file}-timeout.csv {input-file}-invalid.csv {input-file}-valid.csv {input-file}-timing.json
  #[clap(long, value_hint = ValueHint::DirPath, value_parser = Parsers::path, required = true)]
  pub dir_output: Cow<'static, Path>,
  #[clap(long, default_value_t = 25_usize)]
  pub concurrency: usize,
  /// per item
  #[clap(
    long,
    default_value_t = 120_u64,
    value_parser = Parsers::parse_duration_above_0
  )]
  pub timeout_seconds: u64,
  #[clap(long, value_delimiter = ',')]
  pub names_email_role_based: Option<Vec<CowStr>>,
}

#[derive(Args, Debug, Clone, Eq, PartialEq)]
pub struct CommandCheckStringOptions {
  /// separated by space, coma or semicolon
  #[clap(long, required = true)]
  pub input: CowStr,
  #[clap(long, default_value_t = 25_usize)]
  pub concurrency: usize,
  /// per item
  #[clap(
    long,
    default_value_t = 120_u64,
    value_parser = Parsers::parse_duration_above_0
  )]
  pub timeout_seconds: u64,
  #[clap(long, value_delimiter = ',')]
  pub names_email_role_based: Option<Vec<CowStr>>,
}

#[derive(Args, Debug, Clone, Eq, PartialEq)]
pub struct CommandEmptyOptions {}

#[derive(Subcommand, Debug, Clone, Eq, PartialEq)]
pub enum CLISubCommands {
  #[clap(name = "check-dir")]
  CheckDir(CommandCheckDirOptions),
  #[clap(name = "check-file")]
  CheckFile(CommandCheckFileOptions),
  #[clap(name = "check-string")]
  CheckString(CommandCheckStringOptions),
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
