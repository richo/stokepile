use chrono::prelude::*;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct StagedFile {
    pub content_path: PathBuf,
    pub manifest_path: PathBuf,
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct UploadDescriptor {
    pub path: RemotePathDescriptor,
    pub device_name: String,
    pub content_hash: [u8; 32],
    pub size: u64,
}

#[derive(Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum RemotePathDescriptor {
    DateTime {
        capture_time: DateTime<Local>,
        extension: String,
    },
    SpecifiedPath {
        path: PathBuf,
    },
}
