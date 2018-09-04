extern crate serde_json;
extern crate hashing_copy;
extern crate sha2;
extern crate regex;
extern crate chrono;
extern crate walkdir;

use chrono::prelude::*;
use regex::{RegexSetBuilder};
use std::collections::HashSet;
use std::io::Read;
use std::fs::{self, File};
use std::path::{Path,PathBuf};
use std::os::unix::ffi::OsStrExt;
use super::staging::{Staging, UploadDescriptor, UploadableFile};
use super::peripheral::MountablePeripheral;
use failure::Error;
use walkdir::WalkDir;

#[derive(Debug)]
pub struct MassStorage {
    // TODO(richo) privatise these
    pub name: String,
    pub path: PathBuf,
    pub extensions: HashSet<String>,
}

pub struct MassStorageFile {
    capturedatetime: DateTime<Local>,
    file: File,
    extension: String,
}

impl UploadableFile for MassStorageFile {
    type Reader = File;

    fn extension(&self) -> &str {
        &self.extension
    }

    fn capture_datetime(&self) -> Result<DateTime<Local>, chrono::ParseError> {
        Ok(self.capturedatetime)
    }

    fn reader(&mut self) -> &mut File {
        &mut self.file
    }
}

impl Staging for MassStorage {
    type FileType = MassStorageFile;

    fn files(&self) -> Result<Vec<MassStorageFile>, Error> {
        let mut out = vec![];
        for entry in WalkDir::new(&self.path) {
            let entry = entry?;
            if entry.file_type().is_dir() { continue }

            let path = entry.path();
            if let Some(ext) = path.extension() {
                let extension = ext.to_str().unwrap().to_lowercase();
                if ! self.extensions.contains(&extension) { continue }

                out.push(MassStorageFile {
                    capturedatetime: path.metadata()?.modified()?.into(),
                    file: File::open(path)?,
                    extension,
                });
            }
        }
        Ok(out)
    }
}

impl MountablePeripheral for MassStorage {
    fn path(&self) -> &PathBuf {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extensions() -> HashSet<String> {
        let mut set = HashSet::new();
        set.insert("mp4".into());
        set
    }

    #[test]
    fn test_mass_storage_loads_files() {
        let mass_storage = MassStorage{
            name: "data".into(),
            path: "test-data/mass_storage".into(),
            extensions: extensions(),
        };

        let files = mass_storage.files().expect("Couldn't load test files");
        assert_eq!(files.len(), 2);
        for file in files {
            assert_eq!(&file.extension, "mp4");
        }
    }
}
