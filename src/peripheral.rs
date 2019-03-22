use std::fs;
use std::path::PathBuf;

pub trait Peripheral: Sized {
    fn mounted(&self) -> bool;

    /// Returns Some(self) if the flysight is attached, None otherwise
    fn get(self) -> Option<Self> {
        if self.mounted() {
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
    fn mounted(&self) -> bool {
        // Hopefully empty means nothing was written there in the meantime
        if !self.path().exists() {
            return false;
        }
        let files: Vec<_> = fs::read_dir(self.path()).unwrap().collect();
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
    }
}
