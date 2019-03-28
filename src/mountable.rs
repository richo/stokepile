use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;

use failure::Error;
use std::fs;
use crate::config::MountableDeviceLocation;

#[derive(Debug)]
pub struct MountedFilesystem<Un: Unmounter> {
    mountpoint: PathBuf,
    device: PathBuf,
    _marker: std::marker::PhantomData<Un>,
}

impl<Un: Unmounter> MountedFilesystem<Un> {
    pub fn new_externally_mounted(mountpoint: PathBuf) -> MountedFilesystem<NoopMounter> {
        MountedFilesystem {
            mountpoint,
            // TODO(richo) Should we look this up?
            device: PathBuf::new(),
            _marker: std::marker::PhantomData::<NoopMounter>,
        }
    }
}

impl<Un: Unmounter> MountedFilesystem<Un> {
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

pub trait Unmounter {
    fn unmount(&mut self);
}

impl UdisksMounter {
    pub fn mount<U, Un>(device: U) -> Result<MountedFilesystem<Un>, Error>
    where U: AsRef<Path>,
          Un: Unmounter,
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
                    _marker: std::marker::PhantomData::<Un>,
                });
            }
        }
        bail!("Failed to mount: {}", String::from_utf8_lossy(&child.stderr));
    }
}

impl Unmounter for UdisksMounter {
    fn unmount(&mut self) {
        match Command::new("udisksctl")
            .arg("unmount")
            .arg("--no-user-interaction")
            .arg("-b")
            // TODO(richo)
            .arg("...")
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

pub struct NoopMounter;
impl Unmounter for NoopMounter {
    fn unmount(&mut self) {
        info!("Doing nothin!");
    }
}

impl<Un> Drop for MountedFilesystem<Un>
where Un: Unmounter {
    fn drop(&mut self) {
        Un::unmount(self);
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

/// This is a subtrait of `mountable` meant to represent devices that can be mounted as a logical
/// filesystem. Implementers need only supply some information about how to find the device, and
/// inherent impls will take care of getting your device mounted and available.
///
/// For devices which require more handholding, look into implementing the `Mountable` trait.
pub trait MountableFilesystem: Sized {
    type Target: MountableKind<This = Self>;

    fn mount(self) -> Result<Self::Target, Error> {
        let mount = match self.location() {
            MountableDeviceLocation::Label(lbl) => {
                let device = device_for_label(&lbl);
                UdisksMounter::mount(device)?
            },
            MountableDeviceLocation::Mountpoint(_) => unimplemented!(),
        };

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
    type Unmounter: Unmounter;

    fn from_mounted_parts(this: Self::This, mount: MountedFilesystem<Self::Unmounter>) -> Self;
}
