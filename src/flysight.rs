extern crate serde_json;
extern crate hashing_copy;
extern crate regex;
extern crate chrono;

use chrono::prelude::*;
use std::io::Read;
use std::fs::{self, File};
use std::path::{Path,PathBuf};
use std::os::unix::ffi::OsStrExt;
use super::staging::{Staging, UploadableFile, UploadDescriptor};
use super::peripheral::MountablePeripheral;
use failure::Error;

#[derive(Debug)]
pub struct Flysight {
    // TODO(richo) privatise these
    pub name: String,
    pub path: PathBuf,
}

pub struct FlysightFile {
    capturedate: String,
    capturetime: String,
    file: File,
}

impl UploadableFile for FlysightFile {
    type Reader = File;

    fn extension(&self) -> &str {
        "csv"
    }

    fn capture_datetime(&self) -> Result<DateTime<Local>, chrono::ParseError> {
        // Adding time is hard, we'll just allocate our faces off
        let mut datetime = self.capturedate.clone();
        datetime.push_str("/");
        datetime.push_str(&self.capturetime);
        Local.datetime_from_str(&datetime, "%y-%m-%d/%H-%M-%S")
    }

    fn reader(&mut self) -> &mut File {
        &mut self.file
    }
}

impl MountablePeripheral for Flysight {
    fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Flysight {
    fn name(&self) -> &String {
        &self.name
    }
}

impl Staging for Flysight {
    type FileType = FlysightFile;

    fn files(&self) -> Result<Vec<FlysightFile>, Error> {
        lazy_static! {
            static ref DATE: regex::bytes::Regex =
                regex::bytes::Regex::new(r"(?P<year>\d{2})-(?P<month>\d{2})-(?P<day>\d{2})").expect("Failed to compile regex");
            static ref ENTRY: regex::bytes::Regex =
                regex::bytes::Regex::new(r"(?P<hour>\d{2})-(?P<min>\d{2})-(?P<second>\d{2}).[cC][sS][vV]").expect("Failed to compile regex");
        }

        let mut out = vec![];
        let path = Path::new(&self.path);
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            // Enter into directories that are named appropriately
            if entry.file_type()?.is_dir() && DATE.is_match(&entry.file_name().as_bytes()) {
                for file in fs::read_dir(entry.path())? {
                    let file = file?;
                    if file.file_type()?.is_file() && ENTRY.is_match(&file.file_name().as_bytes()) {
                        // Trim the .csv from the end
                        let mut filename = file.file_name().into_string().unwrap();
                        let len = filename.len();
                        filename.truncate(len - 4);
                        out.push(FlysightFile {
                            // TODO(richo) There's actually the very real chance that people will
                            // end up with non utf8 garbage.
                            capturedate: entry.file_name().into_string().unwrap(),
                            capturetime: filename,
                            file: File::open(file.path())?
                        });

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
    extern crate tempfile;

    #[test]
    fn test_flysight_loads_files() {
        let flysight = Flysight {
            name: "data".into(),
            path: "test-data/flysight".into(),
        };

        let files = flysight.files().expect("Couldn't load test files");
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_flysight_parses_dates() {
        let flysight = Flysight {
            name: "data".into(),
            path: "test-data/flysight".into(),
        };

        let files = flysight.files().expect("Couldn't load test files");
        assert_eq!(files[0].capture_datetime().unwrap(),
                   Local.ymd(2018, 8, 24).and_hms(9, 55, 30));
        assert_eq!(files[1].capture_datetime().unwrap(),
                   Local.ymd(2018, 8, 24).and_hms(10, 39, 58));
        assert_eq!(files[2].capture_datetime().unwrap(),
                   Local.ymd(2018, 8, 24).and_hms(11, 0, 28));
    }

    #[test]
    fn test_staging_works() {
        let dest = tempfile::tempdir().unwrap();
        let path = dest.path();
        let flysight = Flysight {
            name: "data".into(),
            path: "test-data/flysight".into(),
        };
        flysight.stage_files("data", &path);
        // TODO(richo) test harder
        let iter = fs::read_dir(path).unwrap();
        let files: Vec<_> = iter.collect();

        assert_eq!(files.len(), 6);
    }
}
