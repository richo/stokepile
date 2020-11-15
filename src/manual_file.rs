use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::fmt::Debug;
use crate::staging::{StorableFile, RemotePathDescriptor};

use chrono;
use chrono::prelude::*;

use failure::{Error, ResultExt};

#[derive(Debug)]
pub struct ManualFile {
    captured: DateTime<Local>,
    file: File,
    /// Where do we find this file on the filesystem
    source_path: PathBuf,
    /// The name we're going to store it as
    name: String,
    /// The filetype to store as
    extension: String,
}

impl ManualFile {
    /// Construct a ManualFile
    ///
    /// source_path is the path on disk that we can find this file.
    /// dest_path is the path that we will be storing this file.
    ///
    /// This would mean that for a file at `/path/to/event/name/video.ogv` you could create it as:
    /// ```rust
    /// # use stokepile::manual_file::ManualFile;
    /// # use std::path::PathBuf;
    /// ManualFile::from_path(PathBuf::from("/path/to/event/name/video.ogv"));
    /// ```
    ///
    /// Yielding a ManualFile to stage `video.ogv` filed by the mtime of the file. Filename and
    /// path are stored separately on the ManualFile to make intermediate renaming simpler, since
    /// both the source path and file handle are stored elsewhere.
    pub fn from_path<U>(source_path: U) -> Result<ManualFile, Error>
    where U: AsRef<Path> + Debug {

        let source_path = source_path.as_ref().to_path_buf();
        let file = File::open(&source_path)
            .context("Opening local copy for a ManualFile")?;
        let name = source_path.file_stem()
            .and_then(|x| x.to_str())
            .ok_or_else(|| format_err!("error extracting filename, path: {:?}", &source_path))?
            .to_string();
        let extension = source_path.extension()
            .and_then(|x| x.to_str())
            .ok_or_else(|| format_err!("error extracting extension, path: {:?}", &source_path))?
            .to_string();

        let captured = file.metadata()?.modified()?.into();

        Ok(ManualFile {
            captured,
            file,
            source_path,
            name,
            extension,
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
                ManualFile::from_path(entry.path())
                    .unwrap_or_else(|e| panic!("Couldn't get ManualFile: {:?}, {:?}", e, entry.path()))
            })
    }

    pub fn rename(&mut self, name: String) {
        self.name = name;
    }
}

impl StorableFile for ManualFile {
    type Reader = File;

    fn remote_path(&self) -> Result<RemotePathDescriptor, Error> {
        Ok(RemotePathDescriptor::DateName {
            capture_date: self.captured.naive_local().date(),
            name: self.name.clone(),
            extension: self.extension.clone(),
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
        let stager = test_helpers::temp_stager();
        let source = test_helpers::tempdir();

        let path = source.path().join("test-file.ogv");
        let mut test_data = File::create(&path).expect("Test file create");
        assert!(test_data.write_all(b"This is some test data").is_ok());

        let fh = ManualFile::from_path(path).expect("Couldn't create manualfile");
        let desc = fh.descriptor("test-upload");

        stager.stage(fh, "manual").expect("Didn't stage correct");

        // TODO(richo) Assert the staged file actually showed up?
        // TODO(richo) Assert the old one was deleted
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
        let captured: DateTime<Local> = test_data.metadata().unwrap().modified().unwrap().into();
        assert!(test_data.write_all(b"This is some test data").is_ok());

        let mut iter = ManualFile::iter_from(event);
        let mf = iter.next().expect("Couldn't get a test file");
        let desc = mf.descriptor("event name").expect("Couldn't get descriptor");

        assert_eq!(&desc.path,
                   &staging::RemotePathDescriptor::DateName {
                       capture_date: captured.naive_local().date(),
                       name: "video".into(),
                       extension: "ogv".into(),
                   });

        assert_eq!(&desc.device_name,
                   "event name");

        assert!(iter.next().is_none());
    }
}
