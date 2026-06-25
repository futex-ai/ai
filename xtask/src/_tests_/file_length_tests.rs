use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::file_length;

#[test]
fn accepts_rust_file_at_line_limit() {
    let workspace = test_workspace();
    let source = workspace.join("crates/demo/src/lib.rs");
    write_lines(&source, 300);

    file_length::run(&workspace, true).unwrap();
}

#[test]
fn rejects_rust_file_over_line_limit() {
    let workspace = test_workspace();
    let source = workspace.join("xtask/src/main.rs");
    write_lines(&source, 301);

    let error = file_length::run(&workspace, true).unwrap_err().to_string();

    assert!(error.contains("xtask/src/main.rs has 301 lines"));
}

fn test_workspace() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "ai-xtask-file-length-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(path.join("crates/demo/src")).unwrap();
    fs::create_dir_all(path.join("xtask/src")).unwrap();
    path
}

fn write_lines(path: &PathBuf, count: usize) {
    let contents = "line\n".repeat(count);
    fs::write(path, contents).unwrap();
}
