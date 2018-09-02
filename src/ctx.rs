extern crate libusb;

use super::config;
use std::path::PathBuf;

pub struct Ctx {
    pub usb_ctx: libusb::Context,
    pub cfg: config::Config,
    pub staging: PathBuf,
}
