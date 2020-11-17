use std::path::{Path, PathBuf};
use std::fmt::Debug;
use std::fs::{self, File};
use std::io::{self, Read};

use chrono;
use chrono::prelude::*;
use dropbox_content_hasher::DropboxContentHasher;
use crate::formatting;
use failure::{Error, ResultExt};
use hashing_copy;
use serde_json;
use uuid::Uuid;

use crate::config::{MountableDeviceLocation, StagingConfig};
use crate::mountable::{MountedFilesystem, MountableFilesystem, MountableKind, MOUNTABLE_DEVICE_FOLDER};
use crate::reporting::ReportEntryDescription;

pub use stokepile_shared::staging::{
    StagedFile,
    UploadDescriptor,
    RemotePathDescriptor,
    MediaTransform, TrimDetail,
};

impl MountableFilesystem for StagingConfig {
    type Target = MountedStaging;

    fn location(&self) -> &MountableDeviceLocation {
        &self.location
    }
}

impl MountableKind for MountedStaging {
    type This = StagingConfig;

    fn from_mounted_parts(this: Self::This, mount: MountedFilesystem) -> Self {
        MountedStaging {
            staging: this,
            mount,
        }
    }
}

impl StagingLocation for MountedStaging {
    fn relative_path(&self, path: &Path) -> PathBuf {
        self.mount.path()
            .join(MOUNTABLE_DEVICE_FOLDER)
            .join(path)
    }

    fn read_dir(&self) -> Result<fs::ReadDir, io::Error> {
        fs::read_dir(self.mount.path()
                     .join(MOUNTABLE_DEVICE_FOLDER))
    }
}

#[derive(Debug)]
pub struct MountedStaging {
    staging: StagingConfig,
    mount: MountedFilesystem,
}

pub trait StorableFile {
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
            uuid: Uuid::new_v4(),

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

impl<T> StorableFile for T where T: DateTimeUploadable {
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

fn stage_file<T, U>(file: &mut T, destination: &U, name: &str) -> Result<(), Error>
where T: StorableFile,
      U: StagingLocation,
{
    let mut desc = file.descriptor(name)?;

    let staging_name = desc.staging_name();
    let manifest_name = desc.manifest_name();

    let mut options = fs::OpenOptions::new();
    // TODO(richo) create_new would be a lot less scary here.
    // Currently a duplicate file mtime will overwrite data
    let options = options.write(true).create(true).truncate(true);

    let staging_path = destination.path_for_name(&staging_name);
    let manifest_path = destination.path_for_name(&manifest_name);

    info!("Staging {} to {:?}", &staging_name, &staging_path);
    {
        let mut staged = options.open(&staging_path)
            .context(format!("Open staging path: {:?}", &staging_path))?;
        let (size, hash) = hashing_copy::copy_and_hash::<_, _, DropboxContentHasher>(
            file.reader(),
            &mut staged,
            )
            .context("Copying file to staging")?;
        assert_eq!(size, desc.size);
        desc.content_hash.copy_from_slice(&hash);
        info!("Staged {}: shasum={:x} size={}", &staging_name, &hash, formatting::human_readable_size(size));
    } // Ensure that we've closed our staging file

    {
        info!("Manifesting {}", &manifest_name);
        trace!(" To {:?}", manifest_path);
        let mut staged = options.open(&manifest_path)
            .context("Opening manifest")?;
        serde_json::to_writer(&mut staged, &desc)?;
    }

    Ok(())
}

/// The contract of StageableLocation is a directory with a bunch of flat files under it. Doing
/// things other than this will probably panic implementors.
pub trait StagingLocation: Debug + Sync + Send {
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
    fn staged_files(&self) -> Result<Vec<StagedFile>, Error> {
        let mut out = vec![];
        for entry in self.read_dir().context("Reading staged files")? {
            // Find manifests and work backward
            let entry = entry?;
            trace!("Looking at {:?}", entry.path());
            if !is_manifest(&entry.path()) {
                continue;
            }
            let manifest_path = entry.path();
            let content_path = content_path_from_manifest(&manifest_path);

            let manifest = File::open(&manifest_path)
                .context("Reading manifest")?;

            let descriptor: UploadDescriptor = serde_json::from_reader(manifest)?;
            out.push(StagedFile {
                content_path,
                manifest_path,
                descriptor,
                transforms: vec![],
            });
        }
        Ok(out)
    }
}

impl<T: StagingLocation> StagingLocation for Box<T> {
    fn relative_path(&self, path: &Path) -> PathBuf {
        (**self).relative_path(path)
    }

    fn read_dir(&self) -> Result<fs::ReadDir, io::Error> {
        (**self).read_dir()
    }
}

pub trait StagedFileExt {
    fn delete(self) -> Result<(), io::Error>;
    fn content_handle(&self) -> Result<File, io::Error>;
    fn apply_transforms(self) -> Result<StagedFile, (StagedFile, Error)>;
}

impl StagedFileExt for StagedFile {
    fn delete(self) -> Result<(), io::Error> { info!("removing {:?}", &self.manifest_path);
        fs::remove_file(&self.manifest_path)?;
        info!("removing {:?}", &self.content_path);
        fs::remove_file(&self.content_path)?;
        Ok(())
    }

    fn content_handle(&self) -> Result<File, io::Error> {
        File::open(&self.content_path)
    }

    /// Apply the transforms, consuming this StagedFile and returning the new one, or if the
    /// transforms fail returning this unmodified file and the error.
    fn apply_transforms(self) -> Result<StagedFile, (StagedFile, Error)> {
        unimplemented!()
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

#[derive(Debug)]
pub struct Stager<T: StagingLocation> {
    location: T,
    destructive: bool,
}

impl<T: StagingLocation> Stager<T> {
    pub fn destructive(location: T) -> Stager<T> {
        Stager {
            location,
            destructive: true,
        }
    }

    pub fn preserving(location: T) -> Stager<T> {
        Stager {
            location,
            destructive: false,
        }
    }

    pub fn stage<F>(&self, mut file: F, name: &str) -> Result<(), Error>
        where F: StorableFile
    {
        stage_file(&mut file, &self.location, name)?;

        if self.destructive {
            file.delete()?;
        }

        Ok(())
    }

    pub fn staging_location(&self) -> &T {
        &self.location
    }

    #[cfg(test)]
    pub fn into_inner(self) -> T {
        self.location
    }
}
pub trait StageFromDevice: Sized {
    type FileType: StorableFile + Debug;

    /// List all stageable files on this device.
    fn files(&self) -> Result<Vec<Self::FileType>, Error>;

    /// Stage all available files on this device, erasing the device copies as they are staged.
    ///
    /// Returns the number of files staged.
    fn stage_files<T: StagingLocation>(self, name: &str, stager: &Stager<T>) -> Result<usize, Error> {
        let mut i = 0;
        for file in self.files()? {
            stager.stage(file, name)?;
            i += 1;
        }
        self.cleanup()?;
        Ok(i)
    }

    /// As per `stage_files` but returns the underlying object so you can inspect it in a test
    /// setting.
    #[cfg(test)]
    fn stage_files_for_test<T: StagingLocation>(self, name: &str, stager: &Stager<T>) -> Result<Self, Error> {
        for file in self.files()
            .context("Locate files")? {
            stager.stage(file, name)?;
        }
        self.cleanup()?;
        Ok(self)
    }

    fn cleanup(&self) -> Result<(), Error> {
        Ok(())
    }
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
            uuid: Uuid::new_v4(),
        }
    }

    pub fn manual_file<S1, S2>(self, group: PathBuf, name: S1, extension: S2) -> UploadDescriptor
    where S1: Into<String>,
          S2: Into<String> {
        let name = name.into();
        let extension = extension.into();

        UploadDescriptor {
            path: RemotePathDescriptor::SpecifiedPath {
                group, name, extension,
            },
            content_hash: Default::default(),
            device_name: self.device_name,
            size: 0,
            uuid: Uuid::new_v4(),
        }
    }
}

pub trait UploadDescriptorExt {
    fn build(device_name: String) -> UploadDescriptorBuilder;
    #[cfg(test)]
    fn test_descriptor() -> Self;
}

pub trait DescriptorNameable {
    fn staging_name(&self) -> String;
    fn manifest_name(&self) -> String;
    fn remote_path(&self) -> PathBuf;
}

impl UploadDescriptorExt for UploadDescriptor {
    fn build(device_name: String) -> UploadDescriptorBuilder {
        UploadDescriptorBuilder {
            device_name,
        }
    }

    #[cfg(test)]
    fn test_descriptor() -> Self {
        UploadDescriptor {
            path: RemotePathDescriptor::DateTime {
                capture_time: Local.ymd(2018, 8, 26).and_hms(14, 30, 0),
                extension: "mp4".into(),
            },
            device_name: "test-device".into(),
            content_hash: Default::default(),
            size: 1024,
            uuid: Uuid::new_v4(),
        }
    }
}

impl DescriptorNameable for UploadDescriptor {
    fn staging_name(&self) -> String {
        ReportEntryDescription::from(self).staging_name()
    }

    fn manifest_name(&self) -> String {
        ReportEntryDescription::from(self).manifest_name()
    }

    fn remote_path(&self) -> PathBuf {
        ReportEntryDescription::from(self).remote_path()
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
            uuid: Uuid::new_v4(),
        };

        assert_eq!(
            upload.remote_path(),
            PathBuf::from("/2017/11/22/test/15-36-10.mp4".to_string())
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
            uuid: Uuid::new_v4(),
        };

        assert_eq!(
            upload.remote_path(),
            PathBuf::from("/2001/01/02/test/03-04-05.mp4".to_string())
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
            uuid: Uuid::new_v4(),
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
impl StagingLocation for tempfile::TempDir {
    // For tests we allow using TempDirs for staging, although for fairly obvious reasons you're
    // unlikely to want to do this in production

    fn relative_path(&self, path: &Path) -> PathBuf {
        self.path().join(path)
    }

    fn read_dir(&self) -> Result<fs::ReadDir, io::Error> {
        fs::read_dir(self.path())
    }
}

impl<T> StagingLocation for &T where T: StagingLocation {
    fn relative_path(&self, path: &Path) -> PathBuf {
        (*self).relative_path(path)
    }

    fn read_dir(&self) -> Result<fs::ReadDir, io::Error> {
        (*self).read_dir()
    }
}
