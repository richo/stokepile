use std::path::{Path, PathBuf};
use std::fmt::Debug;
use std::fs::{self, File};
use std::io::{self, Read};

use chrono;
use chrono::prelude::*;
use dropbox_content_hasher::DropboxContentHasher;
use crate::formatting;
use failure::Error;
use hashing_copy;
use serde::{Deserialize, Serialize};
use serde_json;

use crate::config::{MountableDeviceLocation, StagingConfig};
use crate::mountable::{PlatformMount, MountableFilesystem, MountableKind};

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

impl MountableFilesystem for StagingConfig {
    type Target = MountedStaging;

    fn location(&self) -> &MountableDeviceLocation {
        &self.location
    }
}

impl MountableKind for MountedStaging {
    type This = StagingConfig;
    type MountKind = PlatformMount;

    fn from_mounted_parts(this: Self::This, mount: Self::MountKind) -> Self {
        MountedStaging {
            staging: this,
            mount,
        }
    }
}

impl StageableLocation for MountedStaging {
    fn relative_path(&self, path: &Path) -> PathBuf {
        self.mount.path().join(path)
    }

    fn read_dir(&self) -> Result<fs::ReadDir, io::Error> {
        fs::read_dir(self.mount.path())
    }
}

#[derive(Debug)]
pub struct MountedStaging {
    staging: StagingConfig,
    mount: PlatformMount,
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

pub fn stage_file<T, U>(mut file: T, destination: &U, name: &str) -> Result<(), Error>
where T: UploadableFile,
      U: StageableLocation,
{
    let mut desc = file.descriptor(name)?;

    let staging_name = desc.staging_name();
    let manifest_name = desc.manifest_name();

    let mut options = fs::OpenOptions::new();
    let options = options.write(true).create(true).truncate(true);

    let staging_path = destination.path_for_name(&staging_name);
    let manifest_path = destination.path_for_name(&manifest_name);

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

/// The contract of StageableLocation is a directory with a bunch of flat files under it. Doing
/// things other than this will probably panic implementors.
pub trait StageableLocation: Debug + Sync + Send {
    /// Return a path relative to this location for the given path.
    ///
    /// It's annoying that these can't be Path's with lifetime bounds that force them not to
    /// outlive their parents, parents
    ///
    /// This API is a little odd in that its consumers are largely responsible for figuring out how
    /// to save things, but consumers of the retrieve API are helped out a lot. Potentially there
    /// should be a single API that gives you a containing object for a file and a manifest, and
    /// cleans them up if you don't commit it?
    fn relative_path(&self, path: &Path) -> PathBuf;

    fn path_for_name(&self, name: &str) -> PathBuf {
        let pb = PathBuf::from(name);
        assert!(pb.is_relative());
        self.relative_path(&pb)
    }

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

    fn read_dir(&self) -> Result<fs::ReadDir, io::Error>;

    // TODO(richo) iterator
    fn staged_files(&self) -> Result<Vec<(StagedFile, UploadDescriptor)>, Error> {
        let mut out = vec![];
        for entry in self.read_dir()? {
            // Find manifests and work backward
            let entry = entry?;
            trace!("Looking at {:?}", entry.path());
            if !is_manifest(&entry.path()) {
                continue;
            }
            let manifest_path = entry.path();
            let content_path = content_path_from_manifest(&manifest_path);

            let manifest = File::open(&manifest_path)?;

            let manifest: UploadDescriptor = serde_json::from_reader(manifest)?;
            out.push((StagedFile {
                content_path,
                manifest_path,
            }, manifest));
        }
        Ok(out)
    }
}

impl<T: StageableLocation> StageableLocation for Box<T> {
    fn relative_path(&self, path: &Path) -> PathBuf {
        (**self).relative_path(path)
    }

    fn read_dir(&self) -> Result<fs::ReadDir, io::Error> {
        (**self).read_dir()
    }
}

#[derive(Debug)]
pub struct StagedFile {
    pub content_path: PathBuf,
    manifest_path: PathBuf,
}

impl StagedFile {
    pub fn delete(self) -> Result<(), io::Error> {
        info!("removing {:?}", &self.manifest_path);
        fs::remove_file(&self.manifest_path)?;
        info!("removing {:?}", &self.content_path);
        fs::remove_file(&self.content_path)?;
        Ok(())
    }

    pub fn content_handle(&self) -> Result<File, io::Error> {
        File::open(&self.content_path)
    }
}

#[derive(Fail, Debug)]
pub enum MountError {
    #[fail(display = "Failed to create mountpoint: {}.", _0)]
    TempDir(io::Error),
    #[fail(display = "Failed to mount device: {}.", _0)]
    Mount(Error),
}

#[derive(Debug)]
pub struct StagingDevice {
    location: MountableDeviceLocation,
}

impl StagingDevice {
    pub fn new(location: MountableDeviceLocation) -> Self {
        StagingDevice {
            location,
        }
    }
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
    fn stage_files<T>(self, name: &str, destination: &T) -> Result<(), Error>
    where
        T: StageableLocation,
    {
        for file in self.files()? {
            stage_file(file, destination, name)?;
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

fn is_manifest(path: &Path) -> bool {
    path.to_str().unwrap().ends_with(".manifest")
}

/// Converts a manifest path back into the filename to set
fn content_path_from_manifest(manifest: &Path) -> PathBuf {
    // TODO(richo) oh god why does this not have tests
    let mut content_path = manifest.to_path_buf();
    let mut string = manifest
        .file_name()
        .unwrap()
        .to_os_string()
        .into_string()
        .unwrap();
    let len = string.len();
    string.truncate(len - 9);

    content_path.set_file_name(string);
    content_path
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

    #[test]
    fn test_absolute_manifest_conversion() {
        let manifest = Path::new("/tmp/foo/bar/butts.manifest");
        let content = content_path_from_manifest(&manifest);
        assert_eq!(PathBuf::from("/tmp/foo/bar/butts".to_string()), content);
    }

    #[test]
    fn test_relative_manifest_conversion() {
        let manifest = Path::new("bar/butts.manifest");
        let content = content_path_from_manifest(&manifest);
        assert_eq!(PathBuf::from("bar/butts".to_string()), content);
    }

    #[test]
    fn test_bare_manifest_conversion() {
        let manifest = Path::new("butts.manifest");
        let content = content_path_from_manifest(&manifest);
        assert_eq!(PathBuf::from("butts".to_string()), content);
    }

    #[test]
    fn test_absolute_manifest_detection() {
        let manifest = Path::new("/tmp/foo/bar/butts.manifest");
        assert_eq!(true, is_manifest(&manifest));
        let manifest = Path::new("/tmp/foo/bar/buttsmanifest");
        assert_eq!(false, is_manifest(&manifest));
        let manifest = Path::new("/tmp/foo/bar/butts.manifes");
        assert_eq!(false, is_manifest(&manifest));
    }

    #[test]
    fn test_relative_manifest_detection() {
        let manifest = Path::new("bar/butts.manifest");
        assert_eq!(true, is_manifest(&manifest));
        let manifest = Path::new("bar/buttsmanifest");
        assert_eq!(false, is_manifest(&manifest));
        let manifest = Path::new("bar/butts.manifes");
        assert_eq!(false, is_manifest(&manifest));
    }

    #[test]
    fn test_bare_manifest_detection() {
        let manifest = Path::new("butts.manifest");
        assert_eq!(true, is_manifest(&manifest));
        let manifest = Path::new("buttsmanifest");
        assert_eq!(false, is_manifest(&manifest));
        let manifest = Path::new("butts.manifes");
        assert_eq!(false, is_manifest(&manifest));
    }
}

#[cfg(test)]
impl StageableLocation for tempfile::TempDir {
    // For tests we allow using TempDirs for staging, although for fairly obvious reasons you're
    // unlikely to want to do this in production

    fn relative_path(&self, path: &Path) -> PathBuf {
        self.path().join(path)
    }

    fn read_dir(&self) -> Result<fs::ReadDir, io::Error> {
        fs::read_dir(self.path())
    }
}

impl<T> StageableLocation for &T where T: StageableLocation {
    fn relative_path(&self, path: &Path) -> PathBuf {
        (*self).relative_path(path)
    }

    fn read_dir(&self) -> Result<fs::ReadDir, io::Error> {
        (*self).read_dir()
    }
}
