use std::collections::HashSet;
use std::fs::{self, File};
use std::path::PathBuf;

use super::peripheral::MountablePeripheral;
use super::staging::{Staging, UploadableFile};

use chrono;
use chrono::prelude::*;
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
    source_path: PathBuf,
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

    fn delete(&mut self) -> Result<(), Error> {
        fs::remove_file(&self.source_path)?;
        Ok(())

    }
}

impl Staging for MassStorage {
    type FileType = MassStorageFile;

    fn files(&self) -> Result<Vec<MassStorageFile>, Error> {
        let mut out = vec![];
        for entry in WalkDir::new(&self.path) {
            let entry = entry?;
            if entry.file_type().is_dir() {
                continue;
            }

            let path = entry.path();
            if let Some(ext) = path.extension() {
                let extension = ext.to_str().unwrap().to_lowercase();
                if !self.extensions.contains(&extension) {
                    continue;
                }

                out.push(MassStorageFile {
                    capturedatetime: path.metadata()?.modified()?.into(),
                    file: File::open(path)?,
                    source_path: path.to_path_buf(),
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
    use test_helpers;

    fn extensions() -> HashSet<String> {
        let mut set = HashSet::new();
        set.insert("mp4".into());
        set
    }

    #[test]
    fn test_mass_storage_loads_files() {
        let mass_storage = MassStorage {
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

    #[test]
    fn test_staging_works() {
        let dest = test_helpers::tempdir();
        let source = test_helpers::test_data("mass_storage");

        let mass_storage = MassStorage {
            name: "data".into(),
            path: source.path().to_path_buf(),
            extensions: extensions(),
        };

        mass_storage.stage_files("data", &dest.path()).unwrap();
        // TODO(richo) test harder
        let iter = fs::read_dir(&dest.path()).unwrap();
        let files: Vec<_> = iter.collect();

        // Two files for the two mp4 files, two files for the manifests
        assert_eq!(files.len(), 4);
    }
}
