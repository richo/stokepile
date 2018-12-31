use std::fs::{self, File};
use std::path::{Path, PathBuf};

use crate::reporting::{ReportEntry, UploadReport, UploadStatus};
use crate::staging;

use failure::Error;
use serde_json;

const MAX_RETRIES: usize = 3;

#[derive(Debug)]
pub enum StorageStatus {
    Success,
    Failure,
}

pub trait StorageAdaptor<T>: Send {
    fn upload(
        &self,
        reader: T,
        manifest: &staging::UploadDescriptor,
    ) -> Result<StorageStatus, Error>;

    fn already_uploaded(&self, manifest: &staging::UploadDescriptor) -> bool;

    fn name(&self) -> String;
}

/// Converts a manifest path back into the filename to set
fn content_path_from_manifest(manifest: &Path) -> PathBuf {
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

fn is_manifest(path: &Path) -> bool {
    path.to_str().unwrap().ends_with(".manifest")
}

pub fn upload_from_staged<T>(
    staged: T,
    adaptors: &[Box<dyn StorageAdaptor<File>>],
) -> Result<UploadReport, Error>
where
    T: AsRef<Path>,
{
    let mut report: UploadReport = Default::default();
    info!("Started upload thread!");
    for entry in fs::read_dir(staged)? {
        // Find manifests and work backward
        let entry = entry?;
        trace!("Looking at {:?}", entry.path());
        if !is_manifest(&entry.path()) {
            continue;
        }
        let manifest_path = entry.path();
        let content_path = content_path_from_manifest(&manifest_path);

        let manifest = File::open(&manifest_path)?;

        let manifest: staging::UploadDescriptor = serde_json::from_reader(manifest)?;

        let results: Vec<_> = adaptors
            .iter()
            .map(|ad| {
                info!("Starting {} adaptor for {:?}", ad.name(), &content_path);
                info!("Checking if file already exists");
                if ad.already_uploaded(&manifest) {
                    info!("File was already uploaded - skipping");
                    return (ad.name(), UploadStatus::AlreadyUploaded);
                }

                info!("File not present upstream - beginning upload");
                // We have inverted the sense of "success" and "failure" from try_for_each
                let result = (0..MAX_RETRIES).try_fold(format_err!("dummy error"), |_, i| {
                    let content = File::open(&content_path).expect("Couldn't open content file");
                    match ad.upload(content, &manifest) {
                        Ok(_resp) => {
                            info!("Upload succeeded");
                            // Returning Err short circuits the iterator
                            None
                        }
                        Err(error) => {
                            error!(
                                "Attempt {} of upload of {:?} failed: {:?}",
                                &i, &content_path, &error
                            );
                            Some(error)
                        }
                    }
                });
                // So we have to pull them apart to flip them
                match result {
                    // The "ok" state means we fell all the way through
                    Some(err) => (ad.name(), UploadStatus::Errored(err)),
                    None => (ad.name(), UploadStatus::Succeeded),
                }
            })
            .collect();

        let entry = ReportEntry::new(manifest, results);
        if entry.is_success() {
            info!("removing {:?}", content_path);
            fs::remove_file(&manifest_path)?;
            fs::remove_file(&content_path)?;
        } else {
            info!("one or more adaptors failed, preserving {:?}", content_path);
        }
        report.record_activity(entry);
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use tempfile;
    use crate::staging::UploadDescriptor;

    /// A storage adaptor that will succeed on the nth attempt
    struct TemporarilyBrokenStorageAdaptor {
        attempts: Cell<usize>,
        successful_attempt: usize,
    }

    impl TemporarilyBrokenStorageAdaptor {
        fn new(tries: usize) -> TemporarilyBrokenStorageAdaptor {
            TemporarilyBrokenStorageAdaptor {
                attempts: Cell::new(0),
                successful_attempt: tries,
            }
        }
    }

    impl StorageAdaptor<File> for TemporarilyBrokenStorageAdaptor {
        fn upload(&self, _: File, _: &staging::UploadDescriptor) -> Result<StorageStatus, Error> {
            let this_attempt = self.attempts.get() + 1;
            self.attempts.set(this_attempt);

            if this_attempt == self.successful_attempt {
                return Ok(StorageStatus::Success);
            } else {
                bail!("Spurious error");
            }
        }

        fn already_uploaded(&self, _: &staging::UploadDescriptor) -> bool {
            false
        }

        fn name(&self) -> String {
            "TemporarilyBrokenStorageAdaptor".to_string()
        }
    }

    #[test]
    fn test_temporarily_broken_uploader_actually_works() {
        let manifest = UploadDescriptor::test_descriptor();
        let uploader = TemporarilyBrokenStorageAdaptor::new(3);
        let buf = tempfile::tempfile().expect("Couldn't create tempfile");
        assert!(uploader.upload(buf, &manifest).is_err());
        let buf = tempfile::tempfile().expect("Couldn't create tempfile");
        assert!(uploader.upload(buf, &manifest).is_err());
        let buf = tempfile::tempfile().expect("Couldn't create tempfile");
        assert!(uploader.upload(buf, &manifest).is_ok());
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
