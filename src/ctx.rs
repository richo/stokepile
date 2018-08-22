extern crate libusb;

use super::config;
use super::dropbox;

pub struct Ctx {
    pub usb_ctx: libusb::Context,
    pub cfg: config::Config,
}
