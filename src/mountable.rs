use std::io;
use std::path::Path;

pub trait Mountable {
    type Mountpoint: AsRef<Path>;

    fn device(&self) -> &Path;

    fn set_mountpoint(&mut self, mountpoint: Self::Mountpoint);

    fn mount(&mut self, mountpoint: Self::Mountpoint) -> Result<(), io::Error> {
        self.set_mountpoint(mountpoint);
        Ok(())
    }
}
