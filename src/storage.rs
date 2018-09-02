extern crate serde_json;

use std::path::{Path,PathBuf};
use std::fs::{self,File};

use failure::Error;
use super::dropbox::DropboxFilesClient;
use super::staging;

/// Converts a manifest path back into the filename to set
fn content_path_from_manifest(manifest: &Path) -> PathBuf {
    let mut content_path = manifest.to_path_buf();
    let mut string = manifest.file_name().unwrap().to_os_string().into_string().unwrap();
    let len = string.len();
    string.truncate(len - 9);

    content_path.set_file_name(string);
    content_path
}

fn is_manifest(path: &Path) -> bool {
    path.to_str().unwrap().ends_with(".manifest")
}

pub fn upload_from_staged<T>(staged: T, adaptor: &DropboxFilesClient) -> Result<(), Error>
    where T: AsRef<Path> {
        info!("Started upload thread!");
        for entry in fs::read_dir(staged)? {
            // Find manifests and work backward
            let entry = entry?;
            trace!("Looking at {:?}", entry.path());
            if ! is_manifest(&entry.path()) {
                continue
            }
            let manifest_path = entry.path();
            let content_path = content_path_from_manifest(&manifest_path);

            let manifest = File::open(&manifest_path)?;
            let content = File::open(&content_path)?;

            let manifest: staging::UploadDescriptor = serde_json::from_reader(manifest)?;

            info!("Uploading {:?} to {:?}", &content_path, &manifest.remote_path());
            adaptor.upload_from_reader(content, &manifest.remote_path())?;

            info!("removing {:?}", content_path);
            fs::remove_file(&manifest_path)?;
            fs::remove_file(&content_path)?;
        }
        Ok(())
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
