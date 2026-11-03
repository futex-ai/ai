//! Recorder CLI provider selection and required credential-independent arguments.

use ai_testkit::provider::Provider;
use clap::Parser;

use crate::cli::{Cli, Command};

#[test]
fn selects_each_provider_and_requires_a_valid_explicit_name() {
    for provider in Provider::ALL {
        let cli = Cli::try_parse_from(["xtask", "record-streams", "--provider", provider.name()])
            .expect("valid provider");
        assert_eq!(cli.command, Command::RecordStreams { provider });
    }
    assert!(Cli::try_parse_from(["xtask", "record-streams"]).is_err());
    assert!(Cli::try_parse_from(["xtask", "record-streams", "--provider", "unknown"]).is_err());
}
