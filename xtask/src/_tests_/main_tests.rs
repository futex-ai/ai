use clap::Parser;

use crate::check;
use crate::cli::{Cli, Command};
use crate::command;

#[test]
fn parses_check_command() {
    let cli = Cli::try_parse_from(["xtask", "check"]).unwrap();

    assert_eq!(cli.command, Command::Check);
}

#[test]
fn parses_file_length_all_flag() {
    let cli = Cli::try_parse_from(["xtask", "rust-file-length-lint", "--all"]).unwrap();

    assert_eq!(cli.command, Command::RustFileLengthLint { all: true });
}

#[test]
fn check_runs_core_cargo_commands() {
    let labels: Vec<String> = check::commands()
        .iter()
        .map(|item| command::command_label(item.program, item.args))
        .collect();

    assert_eq!(
        labels,
        vec![
            "cargo fmt --all -- --check",
            "cargo clippy --workspace --all-targets --all-features",
            "cargo test --workspace --all-features",
        ]
    );
}
