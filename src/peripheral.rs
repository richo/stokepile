extern crate regex;

use super::config::{MassStorageConfig,FlysightConfig};
use failure::Error;

use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

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
        // TODO(richo)
        let mut out = vec![];
        Ok(out)
    }
}

impl Peripheral for FlysightConfig {
    fn attached(&self) -> bool {
        let path = Path::new(&self.mountpoint);
        let dcim = path.join(Path::new("config.txt"));

        path.exists() && dcim.exists()
    }

    fn name(&self) -> &String {
        &self.name
    }

    fn files(&self) -> Result<Vec<File>, Error> {
        lazy_static! {
            static ref DATE: regex::Regex =
                regex::Regex::new(r"(?P<year>\d{2})-(?P<month>\d{2})-(?P<day>\d{2})").expect("Failed to compile regex");
            static ref ENTRY: regex::Regex =
                regex::Regex::new(r"(?P<hour>\d{2})-(?P<min>\d{2})-(?P<second>\d{2}).[cC][sS][vV]").expect("Failed to compile regex");
        }

        let mut out = vec![];
        let path = Path::new(&self.mountpoint);
        println!("{:?}", path);
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            // Enter into directories that are named appropriately
            if entry.file_type()?.is_dir() {
                if let Some(date_captures) = DATE.captures(&entry.file_name().into_string().unwrap()) {
                    for file in fs::read_dir(entry.path())? {
                        let file = file?;
                        if file.file_type()?.is_file() {
                            if let Some(file_captures) = ENTRY.captures(&file.file_name().into_string().unwrap()) {
                                out.push(File::open(file.path())?);
                            }
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
    fn test_flysight_loads_files() {
        let flysight = FlysightConfig {
            name: "data".into(),
            mountpoint: "test-data/flysight".into(),
        };

        let files = flysight.files().expect("Couldn't load test files");
        assert_eq!(files.len(), 3);
    }
}
