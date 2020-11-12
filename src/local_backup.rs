use crate::config::{LocalBackupConfig, MountableDeviceLocation};
use crate::mountable::{MountableFilesystem, MountableKind, MountedFilesystem, MOUNTABLE_DEVICE_FOLDER};
use crate::staging::{self, DescriptorNameable};
use crate::storage::{StorageAdaptor, StorageStatus};
use dropbox_content_hasher;

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::PathBuf;

use failure::Error;

#[derive(Debug)]
pub struct MountedLocalBackup {
    local_backup: LocalBackupConfig,
    mount: MountedFilesystem,
}

impl MountedLocalBackup {
    fn containing_dir(&self, manifest: &staging::UploadDescriptor) -> PathBuf {
        let local = self.local_path(manifest);
        local.parent().unwrap().to_path_buf()
    }

    fn local_path(&self, manifest: &staging::UploadDescriptor) -> PathBuf {
        let root = PathBuf::from("/");
        self.mount.path()
            .join(MOUNTABLE_DEVICE_FOLDER)
            .join(manifest.remote_path().strip_prefix(&root).unwrap())
    }
}

impl MountableFilesystem for LocalBackupConfig {
    type Target = MountedLocalBackup;

    fn location(&self) -> &MountableDeviceLocation {
        &self.location
    }
}

impl MountableKind for MountedLocalBackup {
    type This = LocalBackupConfig;

    fn from_mounted_parts(this: Self::This, mount: MountedFilesystem) -> Self {
        MountedLocalBackup {
            local_backup: this,
            mount,
        }
    }
}

impl<T> StorageAdaptor<T> for MountedLocalBackup
where
    T: Read,
{
    fn already_uploaded(&self, manifest: &staging::UploadDescriptor) -> bool {
        let local_path = self.local_path(&manifest);
        match File::open(&local_path) {
            Ok(mut file) => {
                dropbox_content_hasher::DropboxContentHasher::hash_reader(&mut file)
                    .expect("Couldn't read file to hash")
                    .as_slice() == manifest.content_hash
            },
            Err(_) => {
                // warn!("Couldn't open local file {:?}: {:?}", &local_path, e);
                false
            },
        }
    }

    fn upload(
        &self,
        mut reader: T,
        manifest: &staging::UploadDescriptor,
    ) -> Result<StorageStatus, Error> {
        let containing_dir = self.containing_dir(&manifest);
        let local_path = self.local_path(&manifest);

        // TODO(richo) assert that we're mounted first?
        fs::create_dir_all(&containing_dir)?;

        let mut local_file = File::create(&local_path)?;

        io::copy(&mut reader, &mut local_file)?;


        Ok(StorageStatus::Success)
    }

    fn name(&self) -> String {
        "local backup".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::staging::{UploadDescriptor, UploadDescriptorExt};
    use crate::test_helpers;
    use dropbox_content_hasher::DropboxContentHasher;
    use hashing_copy;

    #[test]
    fn test_containing_dir() {
        let backup_adaptor = LocalBackupConfig {
            location: MountableDeviceLocation::Mountpoint("/test/directory".into()),
        }.mount_for_test();
        let manifest = UploadDescriptor::test_descriptor();

        assert_eq!(backup_adaptor.containing_dir(&manifest),
                   PathBuf::from("/test/directory/stokepile/2018/08/26/test-device"));
    }

    #[test]
    fn test_local_path() {
        let backup_adaptor = LocalBackupConfig {
            location: MountableDeviceLocation::Mountpoint("/test/directory".into()),
        }.mount_for_test();
        let manifest = UploadDescriptor::test_descriptor();

        assert_eq!(backup_adaptor.local_path(&manifest),
                   PathBuf::from("/test/directory/stokepile/2018/08/26/test-device/14-30-00.mp4"));
    }

    #[test]
    fn test_already_uploaded() {
        let tmp = test_helpers::tempdir();
        let adaptor = LocalBackupConfig {
            location: MountableDeviceLocation::Mountpoint(tmp.path().to_path_buf()),
        }.mount_for_test();

        let mut manifest = UploadDescriptor::test_descriptor();
        let reader = "This is some dummy data to stage".to_string();

        let containing_dir = adaptor.containing_dir(&manifest);
        fs::create_dir_all(&containing_dir).expect("Couldn't create containing directory");

        let local_path = adaptor.local_path(&manifest);
        let mut local_file = File::create(local_path).expect("Couldn't create local file");

        let (size, hash) = hashing_copy::copy_and_hash::<_, _, DropboxContentHasher>(
            &mut reader.as_bytes(), &mut local_file).expect("Couldn't copy test data");
        manifest.size = size;
        manifest.content_hash.copy_from_slice(&hash);

        assert!(StorageAdaptor::<&[u8]>::already_uploaded(&adaptor, &manifest));
    }
}
