use std::fs::{self, File};
use std::path::{Path, PathBuf};

use dropbox::DropboxFilesClient;
use reporting::{UploadReport, UploadStatus};
use staging;

use failure::Error;
use serde_json;

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

pub fn upload_from_staged<T>(staged: T, adaptor: &DropboxFilesClient) -> Result<UploadReport, Error>
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
        let content = File::open(&content_path)?;

        let manifest: staging::UploadDescriptor = serde_json::from_reader(manifest)?;

        info!("Checking if file already exists");
        match adaptor.get_metadata(&manifest.remote_path()) {
            Ok(ref metadata) if metadata.content_hash() == &manifest.content_hash => {
                info!("File already exists with correct hash - skipping");
                report.record_activity(UploadStatus::AlreadyUploaded, manifest);
            }
            _ => {
                info!(
                    "Uploading {:?} to {:?}",
                    &content_path,
                    &manifest.remote_path()
                );
                match adaptor.upload(content, &manifest.remote_path()) {
                    Ok(_resp) => {
                        report.record_activity(UploadStatus::Succeeded, manifest);
                    }
                    Err(error) => {
                        error!(
                            "Upload of {:?} failed: {:?}, continuing with next file",
                            &content_path, &error
                        );
                        report.record_activity(UploadStatus::Errored(error), manifest);
                        continue;
                    }
                }
            }
        }

        info!("removing {:?}", content_path);
        fs::remove_file(&manifest_path)?;
        fs::remove_file(&content_path)?;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

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
