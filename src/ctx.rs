extern crate libusb;

use super::config;

pub struct Ctx {
    pub usb_ctx: libusb::Context,
    pub cfg: config::Config,
}
