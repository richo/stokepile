extern crate regex;

use super::config::{MassStorageConfig,FlysightConfig};
use failure::Error;

use std::fs::{self, File};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path,PathBuf};

pub trait Peripheral {
    fn attached(&self) -> bool;
    fn name(&self) -> &String;
    fn files(&self) -> Result<Vec<File>, Error>;
}

impl Peripheral for MassStorageConfig {
    fn attached(&self) -> bool {
        let path = Path::new(&self.mountpoint);
        let dcim = path.join(Path::new("DCIM"));

        path.exists() && dcim.exists()
    }

    fn name(&self) -> &String {
        &self.name
    }

    fn files(&self) -> Result<Vec<File>, Error> {
        lazy_static! {
            static ref VIDEO_PATH: regex::bytes::Regex =
                regex::bytes::Regex::new(r"^\d\d\dGOPRO$").expect("Failed to compile regex");
            static ref VIDEO_FILE: regex::bytes::Regex =
                regex::bytes::Regex::new(r"^GOPR.+\.MP4$").expect("Failed to compile regex");
        }
        let mut out = vec![];
        let mut path = PathBuf::from(&self.mountpoint);
        path.push("DCIM");

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            // Enter into directories that are named appropriately
            if entry.file_type()?.is_dir() {
                // TODO(richo) Are we throwing away the path that we need?
                if let Some(_date_captures) = VIDEO_PATH.captures(&entry.file_name().as_bytes()) {
                    for file in fs::read_dir(entry.path())? {
                        let file = file?;
                        if file.file_type()?.is_file() && VIDEO_FILE.is_match(&file.file_name().as_bytes()) {
                            out.push(File::open(file.path())?);
                        }
                    }
                }
            }
        }
        Ok(out)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mass_storage_loads_files() {
        // TODO(richo) this is super gopro shaped, it should probably just walk the whole thing
        // looking for media
        let mass_storage = MassStorageConfig {
            name: "data".into(),
            mountpoint: "test-data/mass_storage".into(),
        };

        let files = mass_storage.files().expect("Couldn't load test files");
        assert_eq!(files.len(), 1);
    }
}
