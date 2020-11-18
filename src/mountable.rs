use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;

use std::fmt::Debug;

use failure::Error;
use std::fs;
use crate::config::MountableDeviceLocation;

pub const MOUNTABLE_DEVICE_FOLDER: &'static str = "stokepile";

#[derive(Debug)]
pub struct MountedFilesystem {
    mountpoint: PathBuf,
    device: PathBuf,
    mounter: Box<dyn Unmounter>,
}

impl MountedFilesystem {
    /// Creates a MountedFilesystem from an already mounted filesystem. No cleanup or validation
    /// will take place to assert that this is actually what it's meant to be.
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
    /// Returns either a MountedFilesystem or an error failing to mount. This will not prepend any
    /// path information.
    pub fn mount<U>(device: U) -> Result<MountedFilesystem, Error>
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
            if let Some(matches) = regex.captures(&String::from_utf8_lossy(&child.stdout)) {
                let mountpoint = PathBuf::from(matches.get(2).unwrap().as_str());
                info!("Mounted at {:?}", &mountpoint);

                return Ok(MountedFilesystem {
                    mountpoint,
                    device: device.as_ref().to_path_buf(),
                    mounter: Box::new(UdisksMounter{}),
                });
            }
        }
        bail!("Failed to mount: {}", String::from_utf8_lossy(&child.stderr));
    }
}

trait Unmounter: Debug + Sync + Send {
    fn unmount(&mut self, device: &Path);
}

impl Unmounter for UdisksMounter {
    fn unmount(&mut self, device: &Path) {
        info!("Unmounting device at {:?}", &device);
        info!("Syncing first");
        match Command::new("sync")
            .status()
        {
            Ok(status) => {
                if status.success() {
                    info!("sync complete")
                } else {
                    warn!("sync returned {:?}", status.code())
                }
            },
            Err(e) => warn!("sync failed, continuing: {:?}", e),
        }

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
                } else {
                    info!("Successfully umounted");
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
    if pb.exists() {
        info!("Checking if {:?} exists.. found!", &pb);
        true
    } else {
        info!("Checking if {:?} exists.. not found", &pb);
        false
    }
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
            MountableDeviceLocation::Location(path) => MountedFilesystem::new_externally_mounted(path.to_owned())
        };

        Ok(Self::Target::from_mounted_parts(self, mount))
    }

    #[cfg(test)]
    fn mount_for_test(self) -> Self::Target {
        let loc = match self.location() {
            MountableDeviceLocation::Label(_) => panic!("Labels not supported in tests"),
            MountableDeviceLocation::Mountpoint(mp) => mp.clone(),
            MountableDeviceLocation::Location(mp) => mp.clone(),
        };

        let mount = MountedFilesystem::new_externally_mounted(loc);
        Self::Target::from_mounted_parts(self, mount)
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
            MountableDeviceLocation::Location(path) => {
                path.exists()
            }
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

    fn from_mounted_parts(this: Self::This, mount: MountedFilesystem) -> Self;
}

/// This is something of a hack, otherwise we couldn't retain ownership of the tempdir and it'd be
/// garbage collected. In essence, this actually does force the tempdir to be unmounted about when
/// unmount() is called.
#[cfg(test)]
impl Unmounter for crate::test_helpers::TempDirUnmounter {
    fn unmount(&mut self, _: &Path) {
        info!("Not unmounting a tempdir although it should be free'd right about now");
    }
}

#[cfg(test)]
impl From<tempfile::TempDir> for MountedFilesystem {
    fn from(temp: tempfile::TempDir) -> Self {
        use crate::test_helpers::TempDirUnmounter;
        MountedFilesystem {
            mountpoint: temp.path().to_path_buf(),
            device: temp.path().to_path_buf(),
            mounter: Box::new(TempDirUnmounter::from(temp)),
        }
    }
}
