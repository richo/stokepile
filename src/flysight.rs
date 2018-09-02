extern crate serde_json;
extern crate hashing_copy;
extern crate sha2;
extern crate regex;
extern crate chrono;

use chrono::prelude::*;
use std::fs::{self, File};
use std::path::{Path,PathBuf};
use std::os::unix::ffi::OsStrExt;
use super::staging::{Staging, UploadDescriptor};
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

impl FlysightFile {
    pub fn capture_date(&self) -> Result<DateTime<Local>, chrono::ParseError> {
        // Adding time is hard, we'll just allocate our faces off
        let mut datetime = self.capturedate.clone();
        datetime.push_str("/");
        datetime.push_str(&self.capturetime);
        Local.datetime_from_str(&datetime, "%y-%m-%d/%H-%M-%S")
    }
}

impl Flysight {
    fn attached(&self) -> bool {
        let dcim = self.path.join(Path::new("config.txt"));

        self.path.exists() && dcim.exists()
    }

    fn name(&self) -> &String {
        &self.name
    }

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

impl Staging for Flysight {
    // Consumes self, purely because connect does
    fn stage_files<T>(self, name: &str, destination: T) -> Result<(), Error>
    where T: AsRef<Path> {
        for mut file in self.files()? {
            let mut desc = UploadDescriptor {
                capture_time: file.capture_date()?,
                device_name: name.to_string(),
                extension: "csv".to_string(),
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
        assert_eq!(files[0].capture_date().unwrap(),
                   Local.ymd(2018, 8, 24).and_hms(9, 55, 30));
        assert_eq!(files[1].capture_date().unwrap(),
                   Local.ymd(2018, 8, 24).and_hms(10, 39, 58));
        assert_eq!(files[2].capture_date().unwrap(),
                   Local.ymd(2018, 8, 24).and_hms(11, 0, 28));
    }
}
