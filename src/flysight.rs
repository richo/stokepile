use std::cmp::Ordering;
use std::fs::{self, File};
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;

use crate::config::{FlysightConfig, MountableDeviceLocation};
use crate::mountable::{MountedFilesystem, MountableFilesystem, MountableKind};
use crate::staging::{StageFromDevice, DateTimeUploadable};

use chrono;
use chrono::prelude::*;
use failure::{Error, ResultExt};
use regex;

#[derive(Debug)]
pub struct MountedFlysight {
    flysight: FlysightConfig,
    mount: MountedFilesystem,
}

#[derive(Debug)]
pub struct FlysightFile {
    capturedate: String,
    capturetime: String,
    file: File,
    source_path: PathBuf,
}

impl Ord for FlysightFile {
    fn cmp(&self, other: &FlysightFile) -> Ordering {
        use std::cmp::Ordering::*;
        match self.capturedate.cmp(&other.capturedate) {
            Less => Less,
            Greater => Greater,
            Equal => self.capturetime.cmp(&other.capturetime),
        }
    }
}

impl PartialOrd for FlysightFile {
    fn partial_cmp(&self, other: &FlysightFile) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for FlysightFile {
    fn eq(&self, other: &FlysightFile) -> bool {
        self.capturedate == other.capturedate && self.capturetime == other.capturetime
    }
}

impl Eq for FlysightFile {}

impl DateTimeUploadable for FlysightFile {
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

    fn delete(&mut self) -> Result<(), Error> {
        fs::remove_file(&self.source_path)?;
        // TODO(richo)
        // Check if this directory structure is empty now and remove it.
        Ok(())
    }

    fn size(&self) -> Result<u64, Error> {
        Ok(self.file.metadata()?.len())
    }
}

impl MountableFilesystem for FlysightConfig {
    type Target = MountedFlysight;

    fn location(&self) -> &MountableDeviceLocation {
        &self.location
    }
}

impl MountableKind for MountedFlysight {
    type This = FlysightConfig;

    fn from_mounted_parts(this: Self::This, mount: MountedFilesystem) -> Self {
        MountedFlysight {
            flysight: this,
            mount,
        }
    }
}

impl StageFromDevice for MountedFlysight {
    type FileType = FlysightFile;

    fn files(&self) -> Result<Vec<FlysightFile>, Error> {
        lazy_static! {
            static ref DATE: regex::bytes::Regex =
                regex::bytes::Regex::new(r"(?P<year>\d{2})-(?P<month>\d{2})-(?P<day>\d{2})")
                    .expect("Failed to compile regex");
            static ref ENTRY: regex::bytes::Regex = regex::bytes::Regex::new(
                r"(?P<hour>\d{2})-(?P<min>\d{2})-(?P<second>\d{2}).[cC][sS][vV]"
            )
            .expect("Failed to compile regex");
        }

        let mut out = vec![];
        let mount_path = self.mount.path();
        for entry in fs::read_dir(mount_path)? {
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
                            file: File::open(file.path())
                                .context("Opening flysight file")?,
                            source_path: file.path().to_path_buf(),
                        });
                    }
                }
            }
        }
        out.sort_unstable();
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers;

    #[test]
    fn test_flysight_loads_files() {
        let flysight = FlysightConfig {
            name: "data".into(),
            location: MountableDeviceLocation::from_mountpoint("test-data/flysight".into()),
        };
        let mounted = flysight.mount_for_test();

        let files = mounted.files().expect("Couldn't load test files");
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_flysight_parses_dates() {
        let flysight = FlysightConfig {
            name: "data".into(),
            location: MountableDeviceLocation::from_mountpoint("test-data/flysight".into()),
        };
        let mounted = flysight.mount_for_test();

        let files = mounted.files().expect("Couldn't load test files");
        assert_eq!(
            files[0].capture_datetime().unwrap(),
            Local.ymd(2018, 8, 24).and_hms(9, 55, 30)
        );
        assert_eq!(
            files[1].capture_datetime().unwrap(),
            Local.ymd(2018, 8, 24).and_hms(10, 39, 58)
        );
        assert_eq!(
            files[2].capture_datetime().unwrap(),
            Local.ymd(2018, 8, 24).and_hms(11, 0, 28)
        );
    }

    #[test]
    fn test_staging_works() {
        let dest = test_helpers::temp_stager();
        let source = test_helpers::test_data("flysight");

        let flysight = FlysightConfig {
            name: "data".into(),
            location: MountableDeviceLocation::from_mountpoint(source.path().to_path_buf()),
        };
        let mounted = flysight.mount_for_test();


        mounted.stage_files("data", &dest).unwrap();
        // TODO(richo) test harder
        let iter = fs::read_dir(&dest.staging_location()).unwrap();
        let files: Vec<_> = iter.collect();

        assert_eq!(files.len(), 6);
    }
}
