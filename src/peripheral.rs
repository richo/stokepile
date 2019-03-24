use failure::Error;
use std::fs;
use std::path::PathBuf;
use crate::config::MountableDeviceLocation;
use crate::mountable::{ExternallyMounted, UdisksMounter, MountedFilesystem};

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
    pb.exists()
}

pub trait MountableKind {
    fn from_mounted_parts<T>(this: T, mount: MountedFilesystem) -> Self
    where T: MountablePeripheral;
}

pub trait MountablePeripheral: Sized {
    type Output: MountableKind;

    fn mount(self) -> Result<Self::Output, Error> {
        let mount = match &self.location {
            MountableDeviceLocation::Label(lbl) => {
                let device = device_for_label(lbl);
                UdisksMounter::mount(device)?
            },
            MountableDeviceLocation::Mountpoint(_) => unimplemented!(),
        };

        Ok(Self::Output::from_mounted_parts(self, mount))
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
