[![version](https://img.shields.io/crates/v/email-validate)](https://crates.io/crates/email-validate)
[![Rust](https://github.com/beckend/email-validate-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/beckend/email-validate-rs/actions/workflows/rust.yml)
[![minimum rustc version](https://img.shields.io/badge/rustc-1.72-orange.svg)](https://github.com/beckend/email-validate-rs)


Licence is AGPL-3.0 in accordance with https://github.com/reacherhq/check-if-email-exists

# Information

This CLI will either accept `string` input of email addresses or a `file(s)` input(s) or a `directory` containing `.csv` files, omitting duplicates and then it will determine if the email is valid based on logic inside [email check source](./src/modules/email_check.rs).
It will write to output with duplicates, valid and invalid emails amongst other useful information.

A run of 6000 emails across 4 csv files peaked ~190MB RAM, allowing 200 concurrent tasks across all threads.
Note: careful with `--concurrency`, on a linux machine running [systemd-resolved](https://wiki.archlinux.org/title/systemd-resolved) caused DDOS as of 2023-09-22, running [unbound](https://link-url-here.orghttps://wiki.archlinux.org/title/unbound) DNS server worked fine, this might also saturate your internet connection.

# Usage/Installation

## Subcommand check-string
```shell
$ cargo run -- check-string --help

Usage: email-validate check-string [OPTIONS] --input <INPUT>

Options:
      --input <INPUT>                      separated by space, coma or semicolon
      --concurrency <CONCURRENCY>          [default: 25]
      --timeout-seconds <TIMEOUT_SECONDS>  per item [default: 120]
  -h, --help                               Print help
  -V, --version                            Print version
```

![Alt text](./docs/assets/images/cli-check-string.png?raw=true "command check-string sample run")

&nbsp;
&nbsp;
&nbsp;

## Subcommand check-file

```shell
cargo run -- check-file --help

Usage: email-validate check-file [OPTIONS] --file-input <FILE_INPUT> --dir-output <DIR_OUTPUT>

Options:
      --file-input <FILE_INPUT>            separated by newline, space, coma or semicolon
      --dir-output <DIR_OUTPUT>            Directory to output {input-file}-timeout.csv {input-file}-invalid.csv {input-file}-valid.csv
                                           {input-file}-timing.json
      --concurrency <CONCURRENCY>          [default: 25]
      --timeout-seconds <TIMEOUT_SECONDS>  per item [default: 120]
  -h, --help                               Print help
  -V, --version                            Print version
```

`--file-input` may be repeat for multiple files.
Hitting Esc/CTRL+C/q will abort quit the CLI progress.
The output will be a TUI progress screen identical like the subcommand `check-dir`.

&nbsp;
&nbsp;
&nbsp;

## Subcommand check-dir

```shell
cargo run -- check-dir --help

Usage: email-validate check-dir [OPTIONS] --dir-input <DIR_INPUT> --dir-output <DIR_OUTPUT>

Options:
      --dir-input <DIR_INPUT>              Directory containing .cvs files, it's walked recursively, the emails may be separated by newline, space, coma or
                                           semicolon
      --dir-output <DIR_OUTPUT>            Directory to output {input-file}-timeout.csv {input-file}-invalid.csv {input-file}-valid.csv
                                           {input-file}-timing.json
      --concurrency <CONCURRENCY>          [default: 25]
      --timeout-seconds <TIMEOUT_SECONDS>  per item [default: 120]
  -h, --help                               Print help
  -V, --version                            Print version
```

Hitting `Esc/CTRL+c/q` will process abort the CLI progress.

![Alt text](./docs/assets/images/cli-check-dir.png?raw=true "command check-dir sample run")
