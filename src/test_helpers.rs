use tempfile;
use walkdir;

use std::fs;
use std::path::PathBuf;

/// Copy data from the test-data directory to a tempdir, then return the owned TestDir object to
/// the caller for use in tests that will modify the filesystem.
pub(crate) fn test_data(suffix: &str) -> tempfile::TempDir {
    lazy_static! {
        static ref TEST_DATA: PathBuf = PathBuf::from("test-data");
    }

    let source = tempfile::tempdir().unwrap();
    let root = TEST_DATA.join(suffix);
    for entry in walkdir::WalkDir::new(&root) {
        let entry = entry.unwrap();
        let target = source
            .path()
            .join(entry.path().strip_prefix(&root).unwrap());
        if entry.file_type().is_dir() {
            let _ = fs::create_dir(&target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
    source
}

pub(crate) fn tempdir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}
