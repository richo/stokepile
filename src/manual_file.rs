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
