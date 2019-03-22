use std::path::{Path, PathBuf};
use std::process::Command;

use failure::Error;
use regex::Regex;

#[derive(Debug)]
pub struct MountedFilesystem {
    mountpoint: PathBuf,
    device: PathBuf,
    mounter: Box<dyn Mounter>,
}

impl MountedFilesystem {
    fn new_externally_mounted(mountpoint: PathBuf) -> MountedFilesystem {
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
            .arg(device)
            .spawn()?;

        let regex = Regex::new(r"^Mounted (.+) at (.+)\.");

        let ret = child.wait_with_output()?;
        if ret.status.success() {
            if let Some(matches) = regex.captures(ret.stdout) {
                return Ok(MountedFilesystem {
                    mountpoint: PathBuf::from(matches.get(2)),
                    device: device.to_path_buf(),
                    mounter: Box::new(UdisksMounter{}),
                });
            }
        }
        bail!("Failed to mount: {}", ret.stderr);
    }
}

trait Mounter {
    fn unmount(&mut self, fs: &MountedFilesystem);
}

impl Mounter for UdisksMounter {
    fn unmount(&mut self, fs: &MountedFilesystem) {
        match Command::new("udisksctl")
            .arg("unmount")
            .arg("-b")
            .arg(fs.device)
            .spawn()
        {
            Ok(child) => {
                let ret = child.wait_with_output();
                if !ret.status.success() {
                    error!("Couldn't unmount device: {}", ret.stderr);
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
    fn unmount(&mut self, fs: &MountedFilesystem) {
        info!("Doing nothing because this was mounted when we got here");
    }
}

impl Drop for MountedFilesystem {
    fn drop(&mut self) {
        self.mounter.unmount(&self.device);
    }
}
