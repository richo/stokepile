extern crate chrono;
use chrono::prelude::*;
use failure::Error;

use std::path::Path;
use std::path::PathBuf;

pub trait Staging {
    fn stage_files<T>(self, name: &str, destination: T) -> Result<(), Error>
    where T: AsRef<Path>;
    // TODO(richo) This can actually be in the trait, and then we just make files() implement the
    // interfaces we need.
}

#[derive(Debug,Serialize,Deserialize)]
pub struct UploadDescriptor {
    pub capture_time: DateTime<Local>,
    pub device_name: String,
    pub extension: String,
    pub sha2: [u8; 32],
    pub size: u64,
}

impl UploadDescriptor {
    pub fn staging_name(&self) -> String {
        format!("{}-{}.{}", self.device_name, self.capture_time, self.extension)
    }

    pub fn manifest_name(&self) -> String {
        format!("{}.manifest", self.staging_name())
    }

    pub fn remote_path(&self) -> PathBuf {
        format!("/{}/{}/{}.{}",
                self.date_component(),
                self.device_name,
                self.time_component(),
                self.extension
                ).into()
    }

    fn date_component(&self) -> String {
        self.capture_time.format("%y-%m-%d").to_string()
    }

    fn time_component(&self) -> String {
        self.capture_time.format("%H-%M-%S").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formats_correctly() {
        let datetime = Local.ymd(2017, 11, 22).and_hms(15, 36, 10);
        let path = PathBuf::from("/path/to/whatever");

        let upload = UploadDescriptor {
            capture_time: datetime,
            device_name: "test".to_string(),
            extension: "mp4".to_string(),
            sha2: [0; 32],
            size: 0,
        };

        assert_eq!(upload.remote_path(), PathBuf::from("/17-11-22/test/15-36-10.mp4".to_string()));
    }

    #[test]
    fn test_pads_correctly() {
        let datetime = Local.ymd(2001, 1, 2).and_hms(3, 4, 5);
        let path = PathBuf::from("/path/to/whatever");

        let upload = UploadDescriptor {
            capture_time: datetime,
            device_name: "test".to_string(),
            extension: "mp4".to_string(),
            sha2: [0; 32],
            size: 0,
        };

        assert_eq!(upload.remote_path(), PathBuf::from("/01-01-02/test/03-04-05.mp4".to_string()));
    }
}
