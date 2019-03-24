use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::fmt::Debug;
use crate::staging::{UploadableFile, RemotePathDescriptor};

use chrono;
use chrono::prelude::*;

use failure::Error;

#[derive(Debug)]
pub struct ManualFile {
    captured: DateTime<Local>,
    file: File,
    /// Where do we find this file on the filesystem
    source_path: PathBuf,
    /// Where should we store this file, ignoring the root
    dest_path: PathBuf,
}

impl ManualFile {
    /// Construct a ManualFile
    ///
    /// source_path is the path on disk that we can find this file.
    /// dest_path is the path that we will be storing this file.
    ///
    /// This would mean that for a file at `/path/to/event/name/video.ogv` you could create it as:
    /// ```rust,ignore
    /// ManualFile::from_paths(PathBuf::from("/path/to/event/name/video.ogv"),
    ///                        PathBuf::from("name/video.ogv"));
    /// ```
    // TODO(richo) Is there some better way to express this in the API without all the duplication?
    fn from_paths<U, T>(source_path: U, dest_path: T) -> Result<ManualFile, Error>
    where U: AsRef<Path> + Debug,
          T: AsRef<Path> + Debug {

        assert!(source_path.as_ref().ends_with(&dest_path), "source_path: {:?}, dest_path: {:?}", &source_path, &dest_path);
        assert!(dest_path.as_ref().is_relative());

        let source_path = source_path.as_ref().to_path_buf();
        let dest_path = dest_path.as_ref().to_path_buf();
        let file = File::open(&source_path)?;
        let captured = file.metadata()?.modified()?.into();

        Ok(ManualFile {
            captured,
            file,
            source_path,
            dest_path,
        })
    }

    pub fn file(&self) -> &File {
        &self.file
    }

    pub fn iter_from(path: PathBuf) -> impl Iterator<Item=ManualFile> {
        walkdir::WalkDir::new(&path)
            .into_iter()
            .filter(|e| if let Ok(e) = e {
                e.file_type().is_file() &&
                    !e.file_name().to_string_lossy().starts_with("._")
            } else {
                false
            })
            .map(move |e| {
                let entry = e.expect("couldn't get path");
                // I believe this is safe, since we're constructing `entry` from `path` it
                // shouldn't be possible to hit this assertion, except potentially with symlinks?
                let dest = entry.path().strip_prefix(&path)
                    .expect("Couldn't remove prefix");
                ManualFile::from_paths(entry.path(), dest)
                    .unwrap_or_else(|e| panic!("Couldn't get ManualFile: {:?}, {:?}, {:?}", e, entry.path(), &dest))
            })
    }
}

impl UploadableFile for ManualFile {
    type Reader = File;

    fn remote_path(&self) -> Result<RemotePathDescriptor, Error> {
        Ok(RemotePathDescriptor::SpecifiedPath {
            path: self.dest_path.to_path_buf(),
        })
    }

    fn reader(&mut self) -> &mut File {
        &mut self.file
    }

    fn delete(&mut self) -> Result<(), Error> {
        fs::remove_file(&self.source_path)?;
        Ok(())
    }

    fn size(&self) -> Result<u64, Error> {
        Ok(self.file.metadata()?.len())
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::fs;
    use crate::staging;
    use crate::test_helpers;

    #[test]
    fn test_staging_works() {
        let dest = test_helpers::tempdir();
        let source = test_helpers::tempdir();

        let path = source.path().join("test-file.ogv");
        let mut test_data = File::create(&path).expect("Test file create");
        assert!(test_data.write_all(b"This is some test data").is_ok());

        let fh = ManualFile::from_paths(path, PathBuf::from("test-file.ogv")).expect("Couldn't create manualfile");
        let desc = fh.descriptor("test-upload");
        staging::stage_file(fh, &dest, "manual").expect("Didn't stage correct");
    }

    #[test]
    fn test_manualfile_path_interpretation() {
        let temp = test_helpers::tempdir();

        let mut event = temp.path().to_path_buf();
        event.extend(vec!["event name"]);

        let mut day = event.clone();
        day.extend(vec!["person", "day"]);

        fs::create_dir_all(&day).expect("Couldn't create temp dirs");

        let path = day.join("video.ogv");
        let mut test_data = File::create(&path).expect("Test file create");
        assert!(test_data.write_all(b"This is some test data").is_ok());

        let mut iter = ManualFile::iter_from(event);
        let mf = iter.next().expect("Couldn't get a test file");
        let desc = mf.descriptor("event name").expect("Couldn't get descriptor");
        assert_eq!(&desc.path,
                   &staging::RemotePathDescriptor::SpecifiedPath {
                       path: PathBuf::from("person/day/video.ogv")
                   });

        assert_eq!(&desc.device_name,
                   "event name");

        assert!(iter.next().is_none());
    }
}
