extern crate chrono;
use chrono::prelude::*;
use failure::Error;

use std::path::Path;

pub trait Staging {
    fn stage_files<T>(self, name: &str, destination: T) -> Result<(), Error>
    where T: AsRef<Path>;
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
}
