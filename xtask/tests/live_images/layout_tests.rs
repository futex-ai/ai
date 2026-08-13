//! Integration-target module layout tests.

use std::{fs, path::Path};

const XTASK_MANIFEST: &str = include_str!("../../Cargo.toml");

#[test]
fn split_integration_target_uses_a_directory_root() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let legacy_root = manifest_dir.join("tests/live_images.rs");
    let target_root = manifest_dir.join("tests/live_images/mod.rs");

    assert!(
        !legacy_root.exists(),
        "legacy split-test root must be removed"
    );
    assert!(
        target_root.is_file(),
        "directory-root test target is missing"
    );
    assert!(
        XTASK_MANIFEST
            .contains("[[test]]\nname = \"live_images\"\npath = \"tests/live_images/mod.rs\""),
        "xtask must register the directory-root integration target"
    );

    let root_source = fs::read_to_string(target_root).expect("test target root should be readable");
    assert!(!root_source.contains("#[test]"));
    assert!(!root_source.contains("#[tokio::test]"));

    for entry in fs::read_dir(manifest_dir.join("tests/live_images"))
        .expect("test target directory should be readable")
    {
        let path = entry.expect("test target entry should be readable").path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if path.extension().and_then(|extension| extension.to_str()) == Some("rs")
            && file_name != "mod.rs"
        {
            assert!(
                file_name.ends_with("_tests.rs"),
                "split-test leaf `{file_name}` must end in `_tests.rs`"
            );
        }
    }
}
