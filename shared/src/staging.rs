use chrono::prelude::*;
use std::ops::Deref;
use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[derive(Debug, Serialize, Deserialize)]
pub struct StagedFile {
    pub content_path: PathBuf,
    pub manifest_path: PathBuf,
    pub descriptor: UploadDescriptor,
}

impl Deref for StagedFile {
    type Target = UploadDescriptor;

    fn deref(&self) -> &Self::Target {
        &self.descriptor
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct UploadDescriptor {
    pub path: RemotePathDescriptor,
    pub device_name: String,
    pub content_hash: [u8; 32],
    pub size: u64,
    pub uuid: Uuid,
}

impl UploadDescriptor {
    pub fn name(&self) -> String {
        self.path.name()
    }
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

pub trait GroupByDevice {
    fn grouped_by_device<'a>(&'a self) -> HashMap<&'a str, Vec<&'a UploadDescriptor>>;
}

impl GroupByDevice for Vec<UploadDescriptor> {
    fn grouped_by_device<'a>(&'a self) -> HashMap<&'a str, Vec<&'a UploadDescriptor>> {
        let mut out = HashMap::new();
        for d in self.iter() {
            let vec = &mut *out.entry(&d.device_name[..]).or_insert(vec![]);
            vec.push(d);
        }
        out
    }
}

impl RemotePathDescriptor {
    pub fn name(&self) -> String {
        use RemotePathDescriptor::*;
        match self {
            DateTime { capture_time, extension } => {
                format!("{}.{}", capture_time, extension)
            },
            DateName { name, extension, .. } => {
                format!("{}.{}", name, extension)
            },
            SpecifiedPath { path } => {
                path.file_name().expect("file_name()").to_str().expect("as_str()").to_string()
            },
        }
    }
}
