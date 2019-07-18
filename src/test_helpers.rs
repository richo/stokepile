use tempfile;
use walkdir;

use std::fs::{self, File};
use std::io::{Seek, Write};
use std::path::PathBuf;
use chrono::prelude::*;
use failure::Error;

use crate::staging::{StageFromDevice, Stager, DateTimeUploadable};

/// Copy data from the test-data directory to a tempdir, then return the owned TestDir object to
/// the caller for use in tests that will modify the filesystem.
pub(crate) fn test_data(suffix: &str) -> tempfile::TempDir {
    lazy_static! {
        static ref TEST_DATA: PathBuf = PathBuf::from("test-data");
    }

    let source = tempfile::tempdir().unwrap();
    let root = TEST_DATA.join(&suffix);
    for entry in walkdir::WalkDir::new(&root) {
        let entry = entry.expect(&format!("No test data for {:?}", &suffix));
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

pub(crate) struct DummyDataDevice {
    files: Vec<DummyDataFile>,
}

impl StageFromDevice for DummyDataDevice {
    type FileType = DummyDataFile;

    fn files(&self) -> Result<Vec<Self::FileType>, Error> {
        Ok(self.files.clone())
    }
}

impl DummyDataDevice {
    fn new(num_files: usize) -> DummyDataDevice {
        DummyDataDevice {
            files: (0..num_files).map(|_| {
                DummyDataFile::new().expect("Couldn't create dummy data")
            }).collect(),
        }
    }
}

pub(crate) struct DummyDataFile {
    file: File,
    deleted: bool,
}

impl Clone for DummyDataFile {
    fn clone(&self) -> Self {
        DummyDataFile {
            file: self.file.try_clone().expect("Couldn't clone file"),
            deleted: self.deleted,
        }
    }
}

impl DummyDataFile {
    fn new() -> Result<DummyDataFile, Error> {
        let mut file = tempfile::tempfile().expect("Couldn't create tempfile");
        file.write(b"This is some test data").expect("Couldn't write test data");
        file.seek(std::io::SeekFrom::Start(0)).expect("Couldn't rewind test file");

        Ok(DummyDataFile {
            file,
            deleted: false,
        })
    }

    fn is_deleted(&self) -> bool {
        self.deleted
    }
}

impl DateTimeUploadable for DummyDataFile {
    type Reader = File;

    fn extension(&self) -> &str {
        "dummy"
    }

    fn capture_datetime(&self) -> Result<DateTime<Local>, chrono::ParseError> {
        Ok(Local::now())
    }

    fn reader(&mut self) -> &mut Self::Reader {
        &mut self.file
    }

    fn delete(&mut self) -> Result<(), Error> {
        self.deleted = true;
        Ok(())
    }

    fn size(&self) -> Result<u64, Error> {
        Ok(self.file.metadata()?.len())
    }
}

pub(crate) fn staged_data(num_files: usize) -> Result<tempfile::TempDir, Error> {
    lazy_static! {
        static ref TEST_DATA: PathBuf = PathBuf::from("staged-data/staging");
    }

    let data_dir = tempfile::tempdir()?;

    // Create a dummy device
    let device = DummyDataDevice::new(num_files);

    let stager = Stager::destructive(data_dir);

    // Stage it's contents
    device.stage_files("dummy", &stager)?;

    Ok(stager.into_inner())
}

pub(crate) fn temp_stager() -> Stager<tempfile::TempDir> {
    let tempdir = tempfile::tempdir().unwrap();

    Stager::destructive(tempdir)
}

pub(crate) fn tempdir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}
