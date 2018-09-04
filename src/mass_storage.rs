extern crate serde_json;
extern crate hashing_copy;
extern crate sha2;
extern crate regex;
extern crate chrono;
extern crate walkdir;

use chrono::prelude::*;
use regex::{RegexSetBuilder};
use std::collections::HashSet;
use std::fs::{self, File};
use std::path::{Path,PathBuf};
use std::os::unix::ffi::OsStrExt;
use super::staging::{Staging, UploadDescriptor};
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

impl MassStorage {
    pub fn files(&self) -> Result<Vec<MassStorageFile>, Error> {
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

impl Staging for MassStorage {
    // Consumes self, purely because connect does
    fn stage_files<T>(self, name: &str, destination: T) -> Result<(), Error>
    where T: AsRef<Path> {
        for mut file in self.files()? {
            let mut desc = UploadDescriptor {
                capture_time: file.capturedatetime,
                device_name: name.to_string(),
                extension: file.extension,
                sha2: [0; 32],
                // TODO(richo) actual double check sizes
                size: 0,
            };

            let staging_name = desc.staging_name();
            let manifest_name = desc.manifest_name();

            let mut options = fs::OpenOptions::new();
            let options = options.write(true).create_new(true);

            let staging_path = destination.as_ref().join(&staging_name);
            let manifest_path = destination.as_ref().join(&manifest_name);

            info!("Staging {}", &staging_name);
            trace!(" To {:?}", staging_path);
            {
                let mut staged = options.open(&staging_path)?;
                let (size, hash) = hashing_copy::copy_and_hash::<_, _, sha2::Sha256>(&mut file.file, &mut staged)?;
                // assert_eq!(size, desc.size);
                info!("Shasum: {:x}", hash);
                info!("size: {:x}", size);
                desc.sha2.copy_from_slice(&hash);
            } // Ensure that we've closed our staging file

            {
                info!("Manifesting {}", &manifest_name);
                trace!(" To {:?}", manifest_path);
                let mut staged = options.open(&manifest_path)?;
                serde_json::to_writer(&mut staged, &desc)?;
            }

            // Once I'm more confident that I haven't fucked up staging
            // file.delete()
        }

        Ok(())
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
