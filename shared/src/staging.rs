use chrono::prelude::*;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[derive(Debug, Serialize, Deserialize)]
pub struct StagedFile {
    pub content_path: PathBuf,
    pub manifest_path: PathBuf,
    pub descriptor: UploadDescriptor,
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct UploadDescriptor {
    pub path: RemotePathDescriptor,
    pub device_name: String,
    pub content_hash: [u8; 32],
    pub size: u64,
    pub uuid: Uuid,
}

#[derive(Eq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub enum RemotePathDescriptor {
    DateTime {
        capture_time: DateTime<Local>,
        extension: String,
    },
    DateName {
        // Date<Local> does not implement Serialize,Deserialize so we use a datetime, but we
        // discard the time component in favour of the given name.
        capture_date: NaiveDate,
        name: String,
        extension: String,
    },
    SpecifiedPath {
        path: PathBuf,
    },
}
