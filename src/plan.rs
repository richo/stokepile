extern crate chrono;
use chrono::prelude::*;

use std::path::PathBuf;
use std::fmt;

use super::device;
use super::dropbox;
use failure::Error;

use super::ptp_device;

#[derive(Debug)]
enum UploadSource {
    LocalFile(PathBuf),
    PtpFile(ptp_device::GoproFile), // TODO(richo) closure probably?
}

impl UploadSource {
    fn path(&self) -> &str {
        match self {
            UploadSource::LocalFile(path) => path.as_path().to_str().unwrap(),
            UploadSource::PtpFile(file) => file.filename.as_str(),
        }
    }
}
