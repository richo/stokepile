use failure::Error;

#[derive(Debug)]
pub struct Context;

impl Context {
    pub fn new() -> Result<Context, Error> {
        Ok(Context)
    }

    pub fn devices(&self) -> Result<Vec<Device<'_>>, Error> {
        unimplemented!("You shouldn't be calling methods from dummy_libusb");
    }
}

#[derive(Debug)]
pub struct Device<'a> {
    _marker: std::marker::PhantomData<&'a ()>
}

impl<'a> Device<'a> {
    pub fn device_descriptor(&self) -> Result<Descriptor, Error> {
        unimplemented!("You shouldn't be calling methods from dummy_libusb");
    }
}

#[derive(Debug)]
pub struct Descriptor;

impl Descriptor {
    pub fn product_id(&self) -> u16 {
        unimplemented!("You shouldn't be calling methods from dummy_libusb");
    }

    pub fn vendor_id(&self) -> u16 {
        unimplemented!("You shouldn't be calling methods from dummy_libusb");
    }
}
