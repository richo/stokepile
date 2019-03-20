use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::fmt::Debug;
use super::staging::UploadableFile;

use chrono;
use chrono::prelude::*;

use failure::Error;

#[derive(Debug)]
pub struct ManualFile {
    captured: DateTime<Local>,
    extension: String,
    file: File,
    source_path: PathBuf,
}

impl ManualFile {
    pub fn from_path<U>(path: U) -> Result<ManualFile, Error>
    where U: AsRef<Path> + Debug {
        let extension = match path.as_ref().extension().map(|x| x.to_str()) {
            Some(Some(ext)) => ext.to_string(),
            _ => bail!("Couldn't get extension from {:?}", path),
        };
        let source_path = path.as_ref().to_path_buf();
        let file = File::open(path)?;
        let captured = file.metadata()?.modified()?.into();

        Ok(ManualFile {
            captured,
            extension,
            file,
            source_path,
        })
    }

    pub fn file(&self) -> &File {
        &self.file
    }
}

impl UploadableFile for ManualFile {
    type Reader = File;

    fn extension(&self) -> &str {
        &self.extension
    }

    fn capture_datetime(&self) -> Result<DateTime<Local>, chrono::ParseError> {
        Ok(self.captured)
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
    use crate::staging;
    use crate::test_helpers;

    #[test]
    fn test_staging_works() {
        let dest = test_helpers::tempdir();
        let source = test_helpers::test_data("manual-files");

        let path = source.path().join("test-file.ogv");
        let mut test_data = File::create(&path).expect("Test file create");
        assert!(test_data.write_all(b"This is some test data").is_ok());

        let fh = ManualFile::from_path(path).expect("Couldn't create manualfile");
        staging::stage_file(fh, &dest.path(), "manual").expect("Didn't stage correct");
    }
}
