use std::path::{Path, PathBuf};
use std::process::Command;
use std::fmt::Debug;
use std::fs;

use failure::Error;
use regex::Regex;

use crate::config::MountableDeviceLocation;

#[derive(Debug)]
pub struct MountedFilesystem {
    mountpoint: PathBuf,
    device: PathBuf,
    unmounter: Box<dyn Unmounter>,
}

impl MountedFilesystem {
    pub fn new_externally_mounted(mountpoint: PathBuf) -> MountedFilesystem {
        MountedFilesystem {
            mountpoint,
            // TODO(richo) Should we look this up?
            device: PathBuf::new(),
            unmounter: Box::new(ExternallyMounted{}),
        }
    }

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

pub trait Mountable: Sized {
    type Output;

    fn mount_filesystem<T>(&self) -> Result<MountedFilesystem, Error>
    where T: Mounter
    {
        match self.location() {
            MountableDeviceLocation::Label(lbl) => {
                let path = device_for_label(&lbl[..]);
                T::mount(&path)
            },
        }
    }

    fn location(&self) -> MountableDeviceLocation;

    fn is_attached(&self) -> bool {
        match self.location() {
            MountableDeviceLocation::Label(lbl) => {
                attached_by_label(&lbl[..])
            },
            // MountableDeviceLocation::Mountpoint(path) => {
            //     // Hopefully empty means nothing was written there in the meantime
            //     if !path.exists() {
            //         return false;
            //     }
            //     let files: Vec<_> = fs::read_dir(path).unwrap().collect();
            //     if files.is_empty() {
            //         return false;
            //     }

            //     #[cfg(test)]
            //     { // Only allow .gitkeep in tests
            //         use std::ffi::OsStr;
            //         match files.as_slice() {
            //             &[Ok(ref file)] if file.file_name() == OsStr::new(".gitkeep") => return false,
            //             _ => {}
            //         }
            //     }

            //     true
            // },
        }
    }

    /// Get the device if attached, otherwise return None
    fn get(self) -> Option<Self> {
        if self.is_attached() {
            Some(self)
        } else {
            None
        }
    }
}

pub trait Mounter: Debug {
    fn mount<U>(device: U) -> Result<MountedFilesystem, Error>
        where U: AsRef<Path> + Debug;
}

pub trait Unmounter: Debug {
    fn unmount(&mut self, fs: &Path);
}

impl Mounter for UdisksMounter {
    fn mount<U>(device: U) -> Result<MountedFilesystem, Error>
    where U: AsRef<Path> + Debug
    {
        info!("Mounting {:?}", &device);
        let child = Command::new("udisksctl")
            .arg("mount")
            .arg("--no-user-interaction")
            .arg("-b")
            .arg(device.as_ref())
            .output()?;

        let regex = Regex::new(r"^Mounted (.+) at (.+)\.")
            .expect("Couldn't compile regex");

        if child.status.success() {
            let s = String::from_utf8_lossy(&child.stdout);
            if let Some(matches) = regex.captures(&s.into_owned()) {
                return Ok(MountedFilesystem {
                    mountpoint: PathBuf::from(matches.get(2).unwrap().as_str()),
                    device: device.as_ref().to_path_buf(),
                    unmounter: Box::new(UdisksMounter{}),
                });
            }
        }
        bail!("Failed to mount: {}", String::from_utf8_lossy(&child.stderr));
    }
}

impl Unmounter for UdisksMounter {
    fn unmount(&mut self, device: &Path) {
        info!("Unmounting {:?}", &device);
        match Command::new("udisksctl")
            .arg("unmount")
            .arg("--no-user-interaction")
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
    fn mount<U>(device: U) -> Result<MountedFilesystem, Error>
    where U: AsRef<Path> + Debug {
        info!("Doing nothing because this should already be mounted");
        // TODO(richo) ... uhh what do we do here? We're just taking the device and making it be
        // the mountpoint? Should we accept an Option<Mountpoint> or something?
        Ok(MountedFilesystem::new_externally_mounted(device.as_ref().to_path_buf()))
    }
}

impl Unmounter for ExternallyMounted {
    fn unmount(&mut self, _device: &Path) {
        info!("Doing nothing because this was mounted when we got here");
    }
}

impl Drop for MountedFilesystem {
    fn drop(&mut self) {
        self.unmounter.unmount(&self.device);
    }
}

#[cfg(target_os = "linux")]
fn device_for_label(lbl: &str) -> PathBuf {
    let mut pb = PathBuf::from("/dev/disk/by-label");
    pb.push(lbl);
    pb
}

#[cfg(target_os = "macos")]
fn device_for_label(lbl: &str) -> PathBuf {
    // TODO(richo) it looks like `diskutil mount LABEL` actually works here
    let mut pb = PathBuf::from("/Volumes");
    pb.push(lbl);
    pb
}

fn attached_by_label(lbl: &str) -> bool {
    let pb = device_for_label(lbl);
    pb.exists()
}
