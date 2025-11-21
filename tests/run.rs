use std::io::Write;

use assert_cmd::cargo::cargo_bin_cmd;

fn test_path(s: &str) {
    let output = cargo_bin_cmd!("mustcc").arg(s).output().unwrap();

    std::io::stderr()
        .write_all(output.stderr.as_slice())
        .unwrap();

    assert!(output.status.code() == Some(0), "non-zero exit code")
}

#[test]
fn test_001() {
    test_path("tests/ok/001_functions")
}

#[test]
fn test_002() {
    test_path("tests/ok/002_modules")
}
