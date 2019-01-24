use crate::staging;
use crate::storage::{StorageAdaptor, StorageStatus};

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::PathBuf;

use failure::Error;

#[derive(Debug)]
pub struct LocalBackup {
    pub(crate) destination: PathBuf
}

impl<T> StorageAdaptor<T> for LocalBackup
where
    T: Read,
{
    fn already_uploaded(&self, _manifest: &staging::UploadDescriptor) -> bool {
        // TODO(richo) Check if the hashes match
        return false;
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
