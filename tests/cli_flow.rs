use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;

#[test]
fn cli_help_works() {
    let mut cmd = Command::cargo_bin("ghost-env").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Protect secrets"));
}
