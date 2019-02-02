use crate::staging;
use crate::storage::{StorageAdaptor, StorageStatus};
use dropbox_content_hasher;

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::PathBuf;
use digest::Digest;

use failure::Error;

#[derive(Debug)]
pub struct LocalBackup {
    pub(crate) destination: PathBuf
}

impl LocalBackup {
    fn containing_dir(&self, manifest: &staging::UploadDescriptor) -> PathBuf {
        let local = self.local_path(manifest);
        local.parent().unwrap().to_path_buf()
    }

    fn local_path(&self, manifest: &staging::UploadDescriptor) -> PathBuf {
        let root = PathBuf::from("/");
        self.destination.join(manifest.remote_path().strip_prefix(&root).unwrap())
    }
}

impl<T> StorageAdaptor<T> for LocalBackup
where
    T: Read,
{
    fn already_uploaded(&self, manifest: &staging::UploadDescriptor) -> bool {
        let local_path = self.destination.join(manifest.remote_path());
        match File::open(local_path) {
            Ok(mut file) => {
                let mut hasher: dropbox_content_hasher::DropboxContentHasher = Default::default();
                let mut buf: Vec<_> = vec![0; dropbox_content_hasher::BLOCK_SIZE];
                loop {
                    let len = file.read(&mut buf).unwrap();
                    if len == 0 { break; }
                    hasher.input(&buf[..len])
                }
                drop(file);
                hasher.result().as_slice() == manifest.content_hash
            },
            Err(_) => {
                // TODO(richo) We could figure out what's going on here but it's almost certainly
                // that the file doesn't exist
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
    use crate::staging::UploadDescriptor;

    #[test]
    fn test_containing_dir() {
        let backup_adaptor = LocalBackup {
            destination: PathBuf::from("/test/directory"),
        };
        let manifest = UploadDescriptor::test_descriptor();

        assert_eq!(backup_adaptor.containing_dir(&manifest),
                   PathBuf::from("/test/directory/18-08-26/test-device"));
    }

    #[test]
    fn test_local_path() {
        let backup_adaptor = LocalBackup {
            destination: PathBuf::from("/test/directory"),
        };
        let manifest = UploadDescriptor::test_descriptor();

        assert_eq!(backup_adaptor.local_path(&manifest),
                   PathBuf::from("/test/directory/18-08-26/test-device/14-30-00.mp4"));
    }
}
