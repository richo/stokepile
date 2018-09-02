extern crate regex;

use std::fs::{self, File};
use std::path::{Path,PathBuf};
use std::os::unix::ffi::OsStrExt;
use super::staging::Staging;
use failure::Error;

#[derive(Debug)]
pub struct Flysight {
    // TODO(richo) privatise these
    pub name: String,
    pub path: PathBuf,
}

impl Flysight {
    fn attached(&self) -> bool {
        let dcim = self.path.join(Path::new("config.txt"));

        self.path.exists() && dcim.exists()
    }

    fn name(&self) -> &String {
        &self.name
    }

    fn files(&self) -> Result<Vec<File>, Error> {
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
            if entry.file_type()?.is_dir() {
                if let Some(_date_captures) = DATE.captures(&entry.file_name().as_bytes()) {
                    for file in fs::read_dir(entry.path())? {
                        let file = file?;
                        if file.file_type()?.is_file() {
                            if let Some(_file_captures) = ENTRY.captures(&file.file_name().as_bytes()) {
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

impl Staging for Flysight {
    // Consumes self, purely because connect does
    fn stage_files<T>(self, name: &str, destination: T) -> Result<(), Error>
    where T: AsRef<Path> {
        // let mut plan = Vec::new();
        // for file in self.files()? {
        //     let capture_time = parse_gopro_date(&file.capturedate)?;
        //     let size = file.size as u64;
        //     plan.push((file, UploadDescriptor {
        //         capture_time,
        //         device_name: name.to_string(),
        //         // TODO(richo) is this always true?
        //         extension: "mp4".to_string(),
        //         sha2: [0; 32],
        //         size,
        //     }));

        //     let staging_name = desc.staging_name();
        //     let manifest_name = desc.manifest_name();

        //     let mut options = fs::OpenOptions::new();
        //     let options = options.write(true).create_new(true);

        //     let staging_path = destination.as_ref().join(&staging_name);
        //     let manifest_path = destination.as_ref().join(&manifest_name);

        //     info!("Staging {}", &staging_name);
        //     trace!(" To {:?}", staging_path);
        //     {
        //         let mut staged = options.open(&staging_path)?;
        //         let (size, hash) = hashing_copy::copy_and_hash::<_, _, sha2::Sha256>(&mut file.reader(&mut conn), &mut staged)?;
        //         assert_eq!(size, desc.size);
        //         info!("Shasum: {:x}", hash);
        //         info!("size: {:x}", size);
        //         desc.sha2.copy_from_slice(&hash);
        //     } // Ensure that we've closed our staging file

        //     {
        //         info!("Manifesting {}", &manifest_name);
        //         trace!(" To {:?}", manifest_path);
        //         let mut staged = options.open(&manifest_path)?;
        //         serde_json::to_writer(&mut staged, &desc)?;
        //     }

        //     // Once I'm more confident that I haven't fucked up staging
        //     // file.delete()
        // }

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
}
