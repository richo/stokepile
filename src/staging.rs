use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use chrono;
use chrono::prelude::*;
use dropbox_content_hasher::DropboxContentHasher;
use crate::formatting;
use failure::Error;
use hashing_copy;
use serde::{Deserialize, Serialize};
use serde_json;

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


pub trait UploadableFile {
    type Reader: Read;

    fn remote_path(&self) -> Result<RemotePathDescriptor, Error>;
    fn delete(&mut self) -> Result<(), Error>;
    fn size(&self) -> Result<u64, Error>;
    fn reader(&mut self) -> &mut Self::Reader;

    fn descriptor(&self, name: &str) -> Result<UploadDescriptor, Error> {
        Ok(UploadDescriptor {
            path: self.remote_path()?,
            content_hash: [0; 32],
            device_name: name.to_string(),
            size: self.size()?,
        })
    }
}

pub trait DateTimeUploadable {
    type Reader: Read;
    fn extension(&self) -> &str;
    fn capture_datetime(&self) -> Result<DateTime<Local>, chrono::ParseError>;

    fn remote_path(&self) -> Result<RemotePathDescriptor, Error> {
        Ok(RemotePathDescriptor::DateTime {
            capture_time: self.capture_datetime()?,
            extension: self.extension().to_string(),
        })
    }

    fn delete(&mut self) -> Result<(), Error>;
    fn size(&self) -> Result<u64, Error>;
    fn reader(&mut self) -> &mut Self::Reader;
}

impl<T> UploadableFile for T where T: DateTimeUploadable {
    type Reader = T::Reader;

    fn remote_path(&self) -> Result<RemotePathDescriptor, Error> {
        self.remote_path()
    }
    fn delete(&mut self) -> Result<(), Error> {
        self.delete()
    }
    fn size(&self) -> Result<u64, Error> {
        self.size()
    }
    fn reader(&mut self) -> &mut Self::Reader {
        self.reader()
    }
}

pub fn stage_file<T, U>(mut file: T, destination: U, name: &str) -> Result<(), Error>
where T: UploadableFile,
      U: AsRef<Path>,
{
    let mut desc = file.descriptor(name)?;

    let staging_name = desc.staging_name();
    let manifest_name = desc.manifest_name();

    let mut options = fs::OpenOptions::new();
    let options = options.write(true).create(true).truncate(true);

    let staging_path = destination.as_ref().join(&staging_name);
    let manifest_path = destination.as_ref().join(&manifest_name);

    info!("Staging {} to {:?}", &staging_name, &staging_path);
    {
        let mut staged = options.open(&staging_path)?;
        let (size, hash) = hashing_copy::copy_and_hash::<_, _, DropboxContentHasher>(
            file.reader(),
            &mut staged,
            )?;
        assert_eq!(size, desc.size);
        desc.content_hash.copy_from_slice(&hash);
        info!("Staged {}: shasum={:x} size={}", &staging_name, &hash, formatting::human_readable_size(size as usize));
    } // Ensure that we've closed our staging file

    {
        info!("Manifesting {}", &manifest_name);
        trace!(" To {:?}", manifest_path);
        let mut staged = options.open(&manifest_path)?;
        serde_json::to_writer(&mut staged, &desc)?;
    }

    file.delete()?;

    Ok(())
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
        for file in self.files()? {
            stage_file(file, &destination, name)?;
        }

        Ok(())
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct UploadDescriptor {
    pub(crate) path: RemotePathDescriptor,
    pub device_name: String,
    pub content_hash: [u8; 32],
    pub size: u64,
}

#[derive(Debug)]
pub struct UploadDescriptorBuilder {
    device_name: String,
}

impl UploadDescriptorBuilder {
    pub fn date_time(self, capture_time: DateTime<Local>, extension: String) -> UploadDescriptor {
        UploadDescriptor {
            path: RemotePathDescriptor::DateTime {
                capture_time,
                extension,
            },
            content_hash: Default::default(),
            device_name: self.device_name,
            size: 0,
        }
    }

    pub fn manual_file(self, path: PathBuf) -> UploadDescriptor {
        UploadDescriptor {
            path: RemotePathDescriptor::SpecifiedPath {
                path,
            },
            content_hash: Default::default(),
            device_name: self.device_name,
            size: 0,
        }
    }
}

impl UploadDescriptor {
    pub fn build(device_name: String) -> UploadDescriptorBuilder {
        UploadDescriptorBuilder {
            device_name,
        }
    }

    pub fn staging_name(&self) -> String {
        match &self.path {
            RemotePathDescriptor::DateTime {
                capture_time, extension
            } => {
                format!(
                    "{}-{}.{}",
                    &self.device_name, capture_time, extension
                )
            },
            RemotePathDescriptor::SpecifiedPath {
                path
            } => {
                format!(
                    "{}-{}",
                    &self.device_name,
                    path.to_str().expect("path wasn't valid utf8").replace("/", "-"),
                )
            }
        }
    }

    pub fn manifest_name(&self) -> String {
        format!("{}.manifest", self.staging_name())
    }

    pub fn remote_path(&self) -> PathBuf {
        match &self.path {
            RemotePathDescriptor::DateTime {
                capture_time, extension,
            } => {
                format!(
                    "/{}/{}/{}.{}",
                    capture_time.format("%y-%m-%d"),
                    &self.device_name,
                    capture_time.format("%H-%M-%S"),
                    extension,
                ).into()
            },
            RemotePathDescriptor::SpecifiedPath {
                path
            } => {
                let mut buf = PathBuf::from("/");
                buf.push(&self.device_name);
                assert!(!path.is_absolute());
                buf.extend(path);
                buf
            }
        }
    }

    #[cfg(test)]
    pub fn test_descriptor() -> Self {
        UploadDescriptor {
            path: RemotePathDescriptor::DateTime {
                capture_time: Local.ymd(2018, 8, 26).and_hms(14, 30, 0),
                extension: "mp4".into(),
            },
            device_name: "test-device".into(),
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
            path: RemotePathDescriptor::DateTime {
                capture_time: datetime,
                extension: "mp4".to_string(),
            },
            device_name: "test".to_string(),
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
            path: RemotePathDescriptor::DateTime {
                capture_time: datetime,
                extension: "mp4".to_string(),
            },
            device_name: "test".to_string(),
            content_hash: [0; 32],
            size: 0,
        };

        assert_eq!(
            upload.remote_path(),
            PathBuf::from("/01-01-02/test/03-04-05.mp4".to_string())
        );
    }

    #[test]
    fn test_uploaddescriptor_roundtrips_serializtion() {
        let datetime = Local.ymd(2001, 1, 2).and_hms(3, 4, 5);

        let original = UploadDescriptor {
            path: RemotePathDescriptor::DateTime {
                capture_time: datetime,
                extension: "mp4".to_string(),
            },
            device_name: "test".to_string(),
            content_hash: [0; 32],
            size: 0,
        };

        let serialized = serde_json::to_string(&original).expect("Couldn't serialize test vector");
        let hydrated = serde_json::from_str(&serialized).expect("Couldn't deserialize test data");

        assert_eq!(&original, &hydrated);
    }
}
