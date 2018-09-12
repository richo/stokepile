use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;

pub trait Peripheral: Sized {
    fn attached(&self) -> bool;

    /// Returns Some(self) if the flysight is attached, None otherwise
    fn get(self) -> Option<Self> {
        if self.attached() {
            Some(self)
        } else {
            None
        }
    }
}

pub trait MountablePeripheral {
    fn path(&self) -> &PathBuf;
}

impl<T: MountablePeripheral> Peripheral for T {
    fn attached(&self) -> bool {
        // Hopefully empty means nothing was written there in the meantime
        if !self.path().exists() {
            return false;
        }
        let files: Vec<_> = fs::read_dir(self.path()).unwrap().collect();
        if files.len() == 0 {
            return false;
        }

        #[cfg(test)] // Only allow .gitkeep in tests
        match files.as_slice() {
            &[Ok(ref file)] if file.file_name() == OsStr::new(".gitkeep") => return false,
            _ => {}
        }

        return true;
    }
}
