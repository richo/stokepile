use std::fmt::Debug;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

use chrono;
use chrono::prelude::*;
use dropbox_content_hasher::DropboxContentHasher;
use crate::formatting;
use failure::Error;
use hashing_copy;
use serde::Serialize;
use serde_json;

pub trait UploadableFile {
    type Reader: Read;
    fn extension(&self) -> &str;
    fn capture_datetime(&self) -> Result<DateTime<Local>, chrono::ParseError>;
    fn reader(&mut self) -> &mut Self::Reader;
    fn delete(&mut self) -> Result<(), Error>;
    fn size(&self) -> Result<u64, Error>;
}

pub trait StageableLocation: Debug + Sync {
    /// Return a path relative to this location for the given path.
    ///
    /// It's annoying that these can't be Path's with lifetime bounds that force them not to
    /// outlive their parents, parents
    fn relative_path(&self, path: &Path) -> PathBuf;

    fn file_path(&self, desc: &UploadDescriptor) -> PathBuf {
        let name = desc.staging_name();
        let path: &Path = Path::new(&name);
        assert!(path.is_relative());
        self.relative_path(path)
    }

    fn manifest_path(&self, desc: &UploadDescriptor) -> PathBuf {
        let name = desc.manifest_name();
        let path: &Path = Path::new(&name);
        assert!(path.is_relative());
        self.relative_path(path)
    }
}

#[derive(Debug)]
pub struct StagingDirectory {
    path: PathBuf,
}

impl StageableLocation for StagingDirectory {
    fn relative_path(&self, path: &Path) -> PathBuf {
        assert!(path.is_relative());
        self.path.join(&path)
    }
}

impl StagingDirectory {
    pub fn new(path: PathBuf) -> Self {
        StagingDirectory {
            path,
        }
    }
}

#[derive(Debug)]
pub struct StagingDevice {
    device: PathBuf,
    mountpoint: PathBuf,
}

impl Drop for StagingDevice {
    fn drop(&mut self) {
        // TODO(richo) unmount the device, clean up the tempdir.
    }
}

pub trait Staging: Sized {
    type FileType: UploadableFile;

    /// List all stageable files on this device.
    fn files(&self) -> Result<Vec<Self::FileType>, Error>;

    /// Stage all available files on this device, erasing the device copies as they are staged.
    fn stage_files(self, name: &str, destination: &dyn StageableLocation) -> Result<(), Error> {
        for mut file in self.files()? {
            let mut desc = UploadDescriptor {
                capture_time: file.capture_datetime()?,
                device_name: name.to_string(),
                extension: file.extension().to_string(),
                content_hash: [0; 32],
                size: file.size()?,
            };

            let mut options = fs::OpenOptions::new();
            let options = options.write(true).create(true).truncate(true);

            let file_path = destination.file_path(&desc);
            let manifest_path = destination.manifest_path(&desc);

            info!("Staging {:?} to {:?}", &desc, &file_path);
            {
                let mut staged = options.open(&file_path)?;
                let (size, hash) = hashing_copy::copy_and_hash::<_, _, DropboxContentHasher>(
                    file.reader(),
                    &mut staged,
                )?;
                assert_eq!(size, desc.size);
                desc.content_hash.copy_from_slice(&hash);
                info!("Staged {:?}: shasum={:x} size={}", &file_path, &hash, formatting::human_readable_size(size as usize));
            } // Ensure that we've closed our staging file

            {
                info!("Manifesting {:?} to {:?}", &desc, &manifest_path);
                let mut staged = options.open(&manifest_path)?;
                serde_json::to_writer(&mut staged, &desc)?;
            }

            file.delete()?;
        }

        Ok(())
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
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
            capture_time: Local.ymd(2018, 8, 26).and_hms(14, 30, 0),
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

    #[test]
    fn test_uploaddescriptor_roundtrips_serializtion() {
        let datetime = Local.ymd(2001, 1, 2).and_hms(3, 4, 5);

        let original = UploadDescriptor {
            capture_time: datetime,
            device_name: "test".to_string(),
            extension: "mp4".to_string(),
            content_hash: [0; 32],
            size: 0,
        };

        let serialized = serde_json::to_string(&original).expect("Couldn't serialize test vector");
        let hydrated = serde_json::from_str(&serialized).expect("Couldn't deserialize test data");

        assert_eq!(&original, &hydrated);
    }
}

#[cfg(test)]
impl StageableLocation for tempfile::TempDir {
    // For tests we allow using TempDirs for staging, although for fairly obvious reasons you're
    // unlikely to want to do this in production

    fn relative_path(&self, path: &Path) -> PathBuf {
        self.path().join(path)
    }
}
