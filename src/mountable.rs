use std::path::{Path, PathBuf};
use std::process::Command;
use std::fmt::Debug;

use failure::Error;
use regex::Regex;

#[derive(Debug)]
pub struct MountedFilesystem {
    mountpoint: PathBuf,
    device: PathBuf,
    mounter: Box<dyn Mounter>,
}

impl MountedFilesystem {
    pub fn new_externally_mounted(mountpoint: PathBuf) -> MountedFilesystem {
        MountedFilesystem {
            mountpoint,
            // TODO(richo) Should we look this up?
            device: PathBuf::new(),
            mounter: Box::new(ExternallyMounted{}),
        }
    }
}

#[derive(Debug)]
pub struct ExternallyMounted {
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
            .arg(device.as_ref())
            .spawn()?;

        let regex = Regex::new(r"^Mounted (.+) at (.+)\.")
            .expect("Couldn't compile regex");

        let ret = child.wait_with_output()?;
        if ret.status.success() {
            let s = String::from_utf8_lossy(&ret.stdout);
            if let Some(matches) = regex.captures(&s.into_owned()) {
                return Ok(MountedFilesystem {
                    mountpoint: PathBuf::from(matches.get(2).unwrap().as_str()),
                    device: device.as_ref().to_path_buf(),
                    mounter: Box::new(UdisksMounter{}),
                });
            }
        }
        bail!("Failed to mount: {}", String::from_utf8_lossy(&ret.stderr));
    }
}

trait Mounter: Debug {
    fn unmount(&mut self, fs: &Path);
}

impl Mounter for UdisksMounter {
    fn unmount(&mut self, device: &Path) {
        match Command::new("udisksctl")
            .arg("unmount")
            .arg("-b")
            .arg(device)
            .spawn()
        {
            Ok(child) => {
                if let Ok(ret) = child.wait_with_output() {
                    if !ret.status.success() {
                        error!("Couldn't unmount device: {}", String::from_utf8_lossy(&ret.stderr));
                    }
                } else {
                    error!("Couldn't get exit status");
                }
            },
            Err(e) => {
                error!("Couldn't launch unmount: {:?}", e);
                return;
            }

        }

    }
}

impl Mounter for ExternallyMounted {
    fn unmount(&mut self, _device: &Path) {
        info!("Doing nothing because this was mounted when we got here");
    }
}

impl Drop for MountedFilesystem {
    fn drop(&mut self) {
        self.mounter.unmount(&self.device);
    }
}
