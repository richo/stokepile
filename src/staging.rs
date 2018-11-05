use std::fs;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

use chrono;
use chrono::prelude::*;
use dropbox_content_hasher::DropboxContentHasher;
use failure::Error;
use hashing_copy;
use serde_json;

pub trait UploadableFile {
    type Reader: Read;
    fn extension(&self) -> &str;
    fn capture_datetime(&self) -> Result<DateTime<Local>, chrono::ParseError>;
    fn reader(&mut self) -> &mut Self::Reader;
    fn delete(&mut self) -> Result<(), Error>;
}

pub trait Staging: Sized {
    type FileType: UploadableFile;

    /// List all stageable files on this device.
    fn files(&self) -> Result<Vec<Self::FileType>, Error>;

    /// Stage all available files on this device, erasing the device copies as they are staged.
    fn stage_files<T>(self, name: &str, destination: T) -> Result<(), Error>
    where
        T: AsRef<Path>,
    {
        for mut file in self.files()? {
            let mut desc = UploadDescriptor {
                capture_time: file.capture_datetime()?,
                device_name: name.to_string(),
                extension: file.extension().to_string(),
                content_hash: [0; 32],
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
                let (size, hash) = hashing_copy::copy_and_hash::<_, _, DropboxContentHasher>(
                    file.reader(),
                    &mut staged,
                )?;
                // assert_eq!(size, desc.size);
                info!("Shasum: {:x}", hash);
                info!("size: {:x}", size);
                desc.content_hash.copy_from_slice(&hash);
            } // Ensure that we've closed our staging file

            {
                info!("Manifesting {}", &manifest_name);
                trace!(" To {:?}", manifest_path);
                let mut staged = options.open(&manifest_path)?;
                serde_json::to_writer(&mut staged, &desc)?;
            }

            file.delete()?;
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadDescriptor {
    pub capture_time: DateTime<Local>,
    pub device_name: String,
    pub extension: String,
    pub content_hash: [u8; 32],
    pub size: u64,
}

impl UploadDescriptor {
    pub fn staging_name(&self) -> String {
        format!(
            "{}-{}.{}",
            self.device_name, self.capture_time, self.extension
        )
    }

    pub fn manifest_name(&self) -> String {
        format!("{}.manifest", self.staging_name())
    }

    pub fn remote_path(&self) -> PathBuf {
        format!(
            "/{}/{}/{}.{}",
            self.date_component(),
            self.device_name,
            self.time_component(),
            self.extension
        )
        .into()
    }

    fn date_component(&self) -> String {
        self.capture_time.format("%y-%m-%d").to_string()
    }

    fn time_component(&self) -> String {
        self.capture_time.format("%H-%M-%S").to_string()
    }

    #[cfg(test)]
    pub fn test_descriptor() -> Self {
        UploadDescriptor {
            capture_time: Local::now(),
            device_name: "test-device".into(),
            extension: "mp4".into(),
            content_hash: Default::default(),
            size: 1024,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formats_correctly() {
        let datetime = Local.ymd(2017, 11, 22).and_hms(15, 36, 10);

        let upload = UploadDescriptor {
            capture_time: datetime,
            device_name: "test".to_string(),
            extension: "mp4".to_string(),
            content_hash: [0; 32],
            size: 0,
        };

        assert_eq!(
            upload.remote_path(),
            PathBuf::from("/17-11-22/test/15-36-10.mp4".to_string())
        );
    }

    #[test]
    fn test_pads_correctly() {
        let datetime = Local.ymd(2001, 1, 2).and_hms(3, 4, 5);

        let upload = UploadDescriptor {
            capture_time: datetime,
            device_name: "test".to_string(),
            extension: "mp4".to_string(),
            content_hash: [0; 32],
            size: 0,
        };

        assert_eq!(
            upload.remote_path(),
            PathBuf::from("/01-01-02/test/03-04-05.mp4".to_string())
        );
    }
}
