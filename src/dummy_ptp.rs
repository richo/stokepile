pub use failure::Error;

#[derive(Debug)]
pub struct PtpCamera<'c> {
    _phantom: &'c std::marker::PhantomData<()>,
}

impl<'c> PtpCamera<'c> {
    pub fn delete_object(&mut self, _handle: u32, _1: Option<()>) -> Result<(), Error> {
        unimplemented!("You shouldn't be calling methods from dummy_ptp");
    }

    pub fn get_partialobject(&mut self, _handle: u32, _size: u32, _whatever: u32, _1: Option<()>) -> Result<Vec<u8>, Error> {
        unimplemented!("You shouldn't be calling methods from dummy_ptp");
    }

    pub fn get_objecthandles_all(&mut self, _handle: u32, _kind: Option<u32>, _1: Option<()>) -> Result<Vec<u32>, Error> {
        unimplemented!("You shouldn't be calling methods from dummy_ptp");
    }

    pub fn get_objectinfo(&mut self, _handle: u32, _1: Option<()>) -> Result<ObjectInfo, Error> {
        unimplemented!("You shouldn't be calling methods from dummy_ptp");
    }

    pub fn close_session(&mut self, _1: Option<()>) -> Result<(), Error> {
        unimplemented!("You shouldn't be calling methods from dummy_ptp");
    }

    pub fn power_down(&mut self, _1: Option<()>) -> Result<(), Error> {
        unimplemented!("You shouldn't be calling methods from dummy_ptp");
    }
}

#[derive(Debug)]
#[allow(non_snake_case)]
pub struct ObjectInfo {
    pub CaptureDate: String,
    pub ObjectCompressedSize: u32,
    pub ObjectFormat: u16,
}
