///! Extract metadata from a given mp4 file. It looks like the most plausible rust options aren't
///! there yet, so we're shelling out to `ffmpeg` but doing so behind a coherent API so that
///! hopefully one day we can do the better thing.

use failure::Error;
use std::ffi::OsStr;
use std::process::Command;

use gopro_telemetry::gps_parser::{self, Message};

#[derive(Debug)]
pub struct Metadata {
    bytes: Vec<u8>,
}

impl Metadata {
    pub fn parse_as_gopro(&self) -> Result<Vec<Message>, Error> {
        gps_parser::parse(&self.bytes)
            .map_err(|e| format_err!("Nom error: {:?}", &e))
    }
}

pub fn metadata<P: AsRef<OsStr>>(filename: P) -> Result<Metadata, Error> {
    let output = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i").arg(filename)
        .arg("-codec").arg("copy")
        .arg("-map").arg("0:3")
        .arg("-f").arg("rawvideo")
        .arg("-")
        .output()?;
    if !output.status.success() {
        format_err!("ffmpeg failed to execute: {:?}", output.stderr);
    }
    Ok(Metadata { bytes: output.stdout })
}
