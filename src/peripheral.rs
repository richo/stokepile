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
