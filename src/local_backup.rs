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
        let local = manifest.remote_path();
        let containing_dir = self.destination.join(local.parent().unwrap());
        // TODO(richo) assert that we're mounted first?
        fs::create_dir_all(&containing_dir)?;
        let mut local_copy = File::create(self.destination.join(local))?;

        io::copy(&mut reader, &mut local_copy)?;


        Ok(StorageStatus::Success)
    }

    fn name(&self) -> String {
        "local backup".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
