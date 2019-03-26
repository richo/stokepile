use std::path::{Path, PathBuf};
use std::process::Command;

use failure::Error;
use regex::Regex;

use std::fmt::Debug;

#[derive(Debug)]
pub struct MountedFilesystem {
    mountpoint: PathBuf,
    device: PathBuf,
    mounter: Box<dyn Unmounter>,
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

impl MountedFilesystem {
    pub fn path(&self) -> &Path {
        &self.mountpoint
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
            .arg("--no-user-interaction")
            .arg("-b")
            .arg(device.as_ref())
            .output()?;

        let regex = Regex::new(r"^Mounted (.+) at (.+)\.")
            .expect("Couldn't compile regex");

        if child.status.success() {
            if let Some(matches) = regex.captures(&String::from_utf8_lossy(&child.stdout)) {
                return Ok(MountedFilesystem {
                    mountpoint: PathBuf::from(matches.get(2).unwrap().as_str()),
                    device: device.as_ref().to_path_buf(),
                    mounter: Box::new(UdisksMounter{}),
                });
            }
        }
        bail!("Failed to mount: {}", String::from_utf8_lossy(&child.stderr));
    }
}

trait Unmounter: Debug {
    fn unmount(&mut self, device: &Path);
}

impl Unmounter for UdisksMounter {
    fn unmount(&mut self, device: &Path) {
        match Command::new("udisksctl")
            .arg("unmount")
            .arg("--no-user-interaction")
            .arg("-b")
            .arg(device)
            .output()
        {
            Ok(child) => {
                if !child.status.success() {
                    error!("Couldn't unmount device: {}", String::from_utf8_lossy(&child.stderr));
                }
            },
            Err(e) => {
                error!("Couldn't launch unmount: {:?}", e);
                return;
            }

        }

    }
}

impl Unmounter for ExternallyMounted {
    fn unmount(&mut self, _: &Path) {
        info!("Doing nothing because this was mounted when we got here");
    }
}

impl Drop for MountedFilesystem {
    fn drop(&mut self) {
        let MountedFilesystem {
            device,
            mounter,
            ..
        } = self;
        mounter.unmount(device);
    }
}
