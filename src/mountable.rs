use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;

use failure::Error;
use std::fs;
use crate::config::MountableDeviceLocation;

// TODO(richo)
/// The default kind of mount for this platform. This can be overridden, but it's generally unwise.
// pub type PlatformMount = UdisksMount;
// pub type PlatformMounter = UdisksMounter;

#[derive(Debug)]
pub struct Mount<T: Unmounter> {
    data
    unmounter: T,
}

#[derive(Debug)]
pub struct UdisksMount {
    mountpoint: PathBuf,
    device: PathBuf,
    unmounted: bool,
}

impl UdisksMount {
    pub fn path(&self) -> &Path {
        &self.mountpoint
    }
}

#[derive(Debug)]
pub struct UdisksMounter {
}

impl Mounter for UdisksMounter {
    fn mount<U>(loc: MountableDeviceLocation) -> Result<UdisksMount, Error>
    {
        let device = match loc {
            MountableDeviceLocation::Label(lbl) => device_for_label(lbl),
            MountableDeviceLocation::Mountpoint(_) => unimplemented!("UdisksMounter can not mount by mountpoint"),
        };

        let child = Command::new("udisksctl")
            .arg("mount")
            .arg("--no-user-interaction")
            .arg("-b")
            .arg(device)
            .output()?;

        let regex = Regex::new(r"^Mounted (.+) at (.+)\.")
            .expect("Couldn't compile regex");

        if child.status.success() {
            if let Some(matches) = regex.captures(&String::from_utf8_lossy(&child.stdout)) {
                return Ok(UdisksMount {
                    mountpoint: PathBuf::from(matches.get(2).unwrap().as_str()),
                    device: device.as_ref().to_path_buf(),
                    unmounted: false,
                });
            }
        }
        bail!("Failed to mount: {}", String::from_utf8_lossy(&child.stderr));
    }
}

impl Unmounter for UdisksMounter {
    /// Unmount this filesystem, consuming this reference to it.
    fn unmount(mut self) {
        self.inner_unmount()
    }
}

impl UdisksMounter {
    /// Inner unmount unmounts the filesystem.
    fn inner_unmount(&mut self) {
        if self.unmounted {
            return;
        }
        match Command::new("udisksctl")
            .arg("unmount")
            .arg("--no-user-interaction")
            .arg("-b")
            .arg(self.device)
            .output()
        {
            Ok(child) => {
                if !child.status.success() {
                    error!("Couldn't unmount device: {}", String::from_utf8_lossy(&child.stderr));
                }
                self.unmounted = true;
            },
            Err(e) => {
                error!("Couldn't launch unmount: {:?}", e);
                return;
            }

        }

    }
}

#[derive(Debug)]
pub struct ExternalMount {
    mountpoint: PathBuf,
}

impl ExternalMount {
    pub fn mount(mountpoint: PathBuf) -> ExternalMount {
        ExternalMount {
            mountpoint,
        }
    }

    pub fn unmount(self) {
    }
}


impl Drop for UdisksMount {
    fn drop(&mut self) {
        // TODO(richo) figure out what we're gunna do with this, probably some private thing called
        // inner_drop ?
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
    let mut pb = PathBuf::from("/Volumes");
    pb.push(lbl);
    pb
}

fn attached_by_label(lbl: &str) -> bool {
    let pb = device_for_label(lbl);
    info!("Checking if {:?} exists", &pb);
    pb.exists()
}

/// This trait is the core of mountable, however various blanket impls exist to make implementation
/// simpler for the generic case, which we have a lot of.
pub trait Mountable {
    type Target;

    fn mount(self) -> Result<Self::Target, Error>;
}

pub trait Mounter {
    fn mount<T>(loc: MountableDeviceLocation) -> Result<T, Error>
        where T: MountableFilesystem;
}

pub trait Unmounter {
    fn unmount(mut self) {
        self.inner_unmount()
    }
}

/// This is a subtrait of `mountable` meant to represent devices that can be mounted as a logical
/// filesystem. Implementers need only supply some information about how to find the device, and
/// inherent impls will take care of getting your device mounted and available.
///
/// For devices which require more handholding, look into implementing the `Mountable` trait.
pub trait MountableFilesystem: Sized {
    type Target: MountableKind<This = Self>;
    type Mounter: Mounter;

    fn mount(self) -> Result<Self::Target, Error> {
        let mount = Self::Mounter::mount(self.device)?;

        Ok(Self::Target::from_mounted_parts(self, mount))
    }

    #[cfg(platform = "macos")]
    fn mount_filesystem(&self) -> Result<MountedFilesystem, Error> {
        match self.location() {
            MountableDeviceLocation::Label(lbl) => {
                MountedFilesystem::<ExternallyMounted>::new_externally_mounted(device_for_label(lbl))
            },
            MountableDeviceLocation::Mountpoint(path) => {
                unimplemented!();
            },
        }
    }

    #[cfg(platform = "linux")]
    fn mount_filesystem(&self) -> Result<MountedFilesystem<UdisksMounter>, Error> {
        match self.location() {
            MountableDeviceLocation::Label(lbl) => {
                let dev = device_for_label(lbl);
                UdisksMounter::mount(dev)
            },
            MountableDeviceLocation::Mountpoint(path) => {
            },
        }
    }

    fn location(&self) -> &MountableDeviceLocation;

    fn get(self) -> Option<Self> {
        if self.is_attached() {
            Some(self)
        } else {
            None
        }
    }

    fn is_attached(&self) -> bool {
        match self.location() {
            MountableDeviceLocation::Label(lbl) => {
                attached_by_label(&lbl[..])
            },
            MountableDeviceLocation::Mountpoint(path) => {
                // Hopefully empty means nothing was written there in the meantime
                if !path.exists() {
                    return false;
                }
                let files: Vec<_> = fs::read_dir(path).unwrap().collect();
                if files.is_empty() {
                    return false;
                }

                #[cfg(test)]
                { // Only allow .gitkeep in tests
                    use std::ffi::OsStr;
                    match files.as_slice() {
                        &[Ok(ref file)] if file.file_name() == OsStr::new(".gitkeep") => return false,
                        _ => {}
                    }
                }

                true
            },
        }
    }
}

impl<T> Mountable for T where T: MountableFilesystem {
    type Target = T::Target;

    fn mount(self) -> Result<Self::Target, Error> {
        self.mount()
    }
}

pub trait MountableKind: Sized {
    type This: MountableFilesystem;
    type MountKind;

    fn from_mounted_parts(this: Self::This, mount: Self::MountKind) -> Self;
}
