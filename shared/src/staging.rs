use chrono::prelude::*;
use chrono::Duration;
use std::ops::Deref;
use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use failure::Error;

// #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[derive(Debug, Serialize, Deserialize)]
pub struct StagedFile {
    pub content_path: PathBuf,
    pub manifest_path: PathBuf,
    pub descriptor: UploadDescriptor,
}

impl StagedFile {
    pub fn rename(&mut self, new_name: String) {
        self.descriptor.path = self.descriptor.path.renamed(new_name);
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct TrimDetail {
    pub start: u32,
    pub end: u32,
}

impl TrimDetail {
    pub fn start_as_ffmpeg(&self) -> String {
        Self::time_as_ffmpeg(self.start)
    }

    pub fn end_as_ffmpeg(&self) -> String {
        Self::time_as_ffmpeg(self.end)
    }

    fn time_as_ffmpeg(secs: u32) -> String {
        let time = Duration::seconds(secs as i64);
        format!("{}:{}:{}", time.num_hours(), time.num_minutes(), time.num_seconds())
    }
}

pub trait AsTransform {
    fn tweak_name(&self) -> String;
}

impl AsTransform for TrimDetail {
    fn tweak_name(&self) -> String {
        format!("-trim-{}:{}", &self.start, &self.end)
    }
}

pub trait Trimmer {
    fn trim(&self, file: StagedFile, detail: &TrimDetail) -> Result<StagedFile, (StagedFile, Error)>;
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
    pub trim: Option<TrimDetail>,
}

impl UploadDescriptor {
    pub fn name(&self) -> String {
        self.path.name()
    }

    pub fn human_name(&self) -> String {
        self.path.human_name()
    }

    pub fn group(&self) -> String {
        self.path.group()
    }

    pub fn device(&self) -> String {
        // We return a String here just to keep things consistent this alloc is completely
        // unnecessary
        self.device_name.clone()
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
        group: PathBuf,
        name: String,
        extension: String,
    },
}

impl RemotePathDescriptor {
    fn renamed(&self, new_name: String) -> Self {
        use RemotePathDescriptor::*;
        match self {
            DateTime { capture_time, extension } => {
                DateName {
                    capture_date: capture_time.naive_local().date(),
                    name: new_name,
                    extension: extension.clone(),
                }
            },
            DateName { capture_date, name, extension } => {
                DateName {
                    capture_date: capture_date.clone(),
                    name: new_name,
                    extension: extension.clone(),
                }
            },
            SpecifiedPath { group, name, extension } => {
                SpecifiedPath {
                    group: group.clone(),
                    name: new_name,
                    extension: extension.clone(),
                }
            }
        }
    }
}

pub trait DescriptorGrouping {
    fn grouped_by_device<'a>(&'a self) -> HashMap<&'a str, Vec<&'a UploadDescriptor>>;
    fn grouped_by_device_by_group<'a>(&'a self) -> HashMap<&'a str, HashMap<String, Vec<&'a UploadDescriptor>>>;
}

impl DescriptorGrouping for Vec<UploadDescriptor> {
    fn grouped_by_device<'a>(&'a self) -> HashMap<&'a str, Vec<&'a UploadDescriptor>> {
        let mut out = HashMap::new();
        for d in self.iter() {
            let vec = &mut *out.entry(&d.device_name[..]).or_insert(vec![]);
            vec.push(d);
        }
        out
    }

    fn grouped_by_device_by_group<'a>(&'a self) -> HashMap<&'a str, HashMap<String, Vec<&'a UploadDescriptor>>> {
        // TODO(richo) There's probably some way to do this without the intermediate hashmap, maybe
        // do all of these with tuples under the hood and some boilerplate over them that
        // collect()s?
        let mut out = HashMap::new();
        for (device, media) in self.grouped_by_device() {
            let d = &mut *out.entry(device).or_insert(HashMap::new());
            for entry in media {
                let m = &mut *d.entry(entry.group()).or_insert(vec![]);
                m.push(entry)
            }
        }
        out
    }
}

impl RemotePathDescriptor {
    /// The logical group this media belongs to. For things with a date this would be the day of
    /// the recording.
    pub fn group(&self) -> String {
        use RemotePathDescriptor::*;
        match self {
            DateTime { capture_time, .. } => {
                format!("{:04}/{:02}/{:02}", capture_time.year(), capture_time.month(), capture_time.day())
            },
            DateName { name, extension, .. } |
            SpecifiedPath { name, extension, .. } => {
                format!("{}.{}", name, extension)
            },
        }
    }
    /// The logical name of the recording. For named files this is the name including the
    /// extension, for date filed recordings this is the time with the extension.
    pub fn name(&self) -> String {
        format!("{}.{}", self.human_name(), self.extension())
    }

    pub fn human_name(&self) -> String {
        use RemotePathDescriptor::*;
        match self {
            DateTime { capture_time, .. } => {
                format!("{:02}-{:02}-{:02}", capture_time.hour(), capture_time.minute(), capture_time.second())
            },
            DateName { name, .. } |
            SpecifiedPath { name, .. } => {
                name.to_string()
            },
        }
    }

    pub fn extension(&self) -> &str {
        use RemotePathDescriptor::*;
        match self {
            DateTime { extension, .. } |
            DateName { extension, .. } |
            SpecifiedPath { extension, .. } => {
                extension
            },
        }
    }
}
