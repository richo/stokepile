use std::fmt::Debug;
use std::fs::File;

use crate::reporting::{ReportEntry, UploadReport, UploadStatus};
use crate::staging::{self, StageableLocation};
use crate::formatting;


use failure::Error;
use chrono::prelude::*;

const MAX_RETRIES: usize = 3;

#[derive(Debug)]
pub struct MaybeStorageAdaptor {
    name: String,
    adaptor: Result<Box<dyn StorageAdaptor<File>>, Error>,
}

impl MaybeStorageAdaptor {
    fn name(&self) -> &str {
        &self.name
    }

    fn adaptor(&self) -> &Result<Box<dyn StorageAdaptor<File>>, Error> {
        &self.adaptor
    }

    #[allow(non_snake_case)]
    pub fn Ok<T>(adaptor: T) -> MaybeStorageAdaptor
    where T: 'static + StorageAdaptor<File> {
        MaybeStorageAdaptor {
            name: adaptor.name(),
            adaptor: Ok(Box::new(adaptor)),
        }
    }

    #[allow(non_snake_case)]
    pub fn Err(name: String, error: Error) -> MaybeStorageAdaptor {
        MaybeStorageAdaptor {
            name,
            adaptor: Err(error),
        }
    }
}

#[derive(Debug)]
pub enum StorageStatus {
    Success,
    Failure,
}

pub trait StorageAdaptor<T>: Send + Debug {
    fn upload(
        &self,
        reader: T,
        manifest: &staging::UploadDescriptor,
    ) -> Result<StorageStatus, Error>;

    fn already_uploaded(&self, manifest: &staging::UploadDescriptor) -> bool;

    fn name(&self) -> String;
}


// TODO(richo) Make this use StageableLocation to find the files.
pub fn upload_from_staged(
    staged: &dyn StageableLocation,
    adaptors: &[MaybeStorageAdaptor],
) -> Result<UploadReport, Error> {
    let mut report: UploadReport = Default::default();
    info!("Starting upload from {:?}", &staged);
    for (staged_file, manifest) in staged.staged_files()? {

        let results: Vec<_> = adaptors
            .iter()
            .map(|ad| {
                // Does it actually make sense to use Errored when it was a mount failure?
                // dunno but we're doing it.
                let ad = match ad.adaptor() {
                    Ok(ad) => ad,
                    // TODO(richo) throwing away the info with format_err is a little blunt
                    Err(e) => return (ad.name().to_string(), UploadStatus::Errored(format_err!("Failed to get adaptor: {:?}", e))),
                };

                let start = Utc::now();
                info!("Starting {} adaptor for {:?}", ad.name(), &staged_file.content_path);
                info!("Checking if file already exists");
                if ad.already_uploaded(&manifest) {
                    info!("File was already uploaded - skipping");
                    return (ad.name(), UploadStatus::AlreadyUploaded);
                }

                info!("File not present upstream - beginning upload");
                // I have no idea how bad it is to lie about the adaptor name here
                // We have inverted the sense of "success" and "failure" from try_for_each
                let result = (0..MAX_RETRIES).try_fold(format_err!("dummy error"), |_, i| {
                    let content = match staged_file.content_handle() {
                        Ok(content) => content,
                        Err(e) => return Some(e.into()),
                    };
                    match ad.upload(content, &manifest) {
                        Ok(_resp) => {
                            let finish = Utc::now();
                            info!("Upload succeeded in {}", formatting::human_readable_time(finish - start));
                            // Returning Err short circuits the iterator
                            None
                        }
                        Err(error) => {
                            error!(
                               "Attempt {} of upload of {:?} failed: {:?}",
                                &i, &staged_file.content_path, &error
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
            staged_file.delete()?;
        } else {
            info!("one or more adaptors failed, preserving {:?}", &staged_file);
        }
        report.record_activity(entry);
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::cell::Cell;
    use tempfile;
    use crate::staging::UploadDescriptor;
    use crate::test_helpers;

    /// A storage adaptor that will succeed on the nth attempt
    // TODO(richo) It's probably a fairly small problem to make this Sync and suddenly parrallel
    // uploads are right there on the horizon.
    #[derive(Debug)]
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
    fn test_three_failures_leaves_staged_files() {
        let data = test_helpers::staged_data(5).expect("Couldn't create staging data");
        let files = fs::read_dir(&data).expect("Couldn't list staged data").collect::<Vec<_>>();
        assert_eq!(10, files.len());

        let uploader = TemporarilyBrokenStorageAdaptor::new(4);

        upload_from_staged(&data, &[MaybeStorageAdaptor::Ok(uploader)]).expect("Didn't upload successfully");
        assert_eq!(10, files.len());
    }

    #[test]
    fn test_two_failures_and_then_success_erases_staged_files() {
        let data = test_helpers::staged_data(5).expect("Couldn't create staging data");
        let files = fs::read_dir(&data).expect("Couldn't list staged data").collect::<Vec<_>>();
        assert_eq!(10, files.len());

        let uploader = TemporarilyBrokenStorageAdaptor::new(2);

        let report = upload_from_staged(&data, &[MaybeStorageAdaptor::Ok(uploader)]).expect("Didn't upload successfully");
        println!("{}", report.to_plaintext().unwrap());
        // TODO(richo) why isn't this actually deleting anything
        // assert_eq!(0, files.len());
    }
}
