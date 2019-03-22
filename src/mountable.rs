use std::path::{Path, PathBuf};
use std::process::Command;

use failure::Error;
use regex::Regex;

#[derive(Debug)]
pub struct MountedFilesystem {
    mountpoint: PathBuf,
    device: PathBuf,
}

#[derive(Debug)]
pub struct UdisksMounter {
}

impl UdisksMounter {
    pub fn mount<U>(device: U) -> Result<MountedFilesystem, Error>
    where U: AsRef<Path>
    {
        let child = Command::new("udisksctl")
            .arg("mount")
            .arg("-b")
            .arg(device)
            .spawn()?;

        let regex = Regex::new(r"^Mounted (.+) at (.+)\.");

        let ret = child.wait_with_output()?;
        if ret.status.success() {
            if let Some(matches) = regex.captures(ret.stdout) {
                return Ok(MountedFilesystem {
                    mountpoint: PathBuf::from(matches.get(2)),
                    device: device.to_path_buf(),
                });
            }
        }
        bail!("Failed to mount: {}", ret.stderr);
    }
}

impl Drop for MountedFilesystem {
    fn drop(&mut self) {
        let child = Command::new("udisksctl")
            .arg("unmount")
            .arg("-b")
            .arg(self.device)
            .spawn()?;

        let ret = child.wait_with_output();
        if ret.status.success() {

        }
    }
}
