extern crate libusb;
extern crate ptp;

use std::fs::File;
use std::io::prelude::*;
use std::fmt;

use failure::Error;

use super::ctx;

enum GoproObjectFormat {
    Directory = 0x3001,
    File = 0x300d,
}

const GOPRO_VENDOR: u16 = 0x2672;
const GOPRO_MANUFACTURER: &'static str = "GoPro";

#[repr(u16)]
#[derive(Debug)]
pub enum GoproKind {
    Hero4Silver,
}

impl GoproKind {
    fn from_u16(ty: u16) -> Option<GoproKind> {
        use self::GoproKind::*;
        match ty {
            0x000d => Some(Hero4Silver),
            _ => None,
        }
    }
}

// Specialising to PTP devices later might be neat, but for now this is fine
pub struct Gopro<'a> {
    pub kind: GoproKind,
    pub serial: String,
    pub device: libusb::Device<'a>,
}

impl<'a> fmt::Debug for Gopro<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Gopro")
            .field("kind", &self.kind)
            .field("serial", &self.serial)
            .field("device", &"libusb::Device")
            .finish()
    }
}

pub fn locate_gopros(ctx: &ctx::Ctx) -> Result<Vec<Gopro>, Error> {
    let mut res = vec![];

    for mut device in ctx.usb_ctx.devices()?.iter() {
        let device_desc = device.device_descriptor().unwrap();

        if device_desc.vendor_id() != GOPRO_VENDOR {
            continue
        }

        // We'll just use the Manufacturer tag in the PtpDevice

        let mut camera = ptp::PtpCamera::new(&device)?;
        let info = camera.get_device_info(None)?;

        if info.Manufacturer != GOPRO_MANUFACTURER {
            continue
        }

        match GoproKind::from_u16(device_desc.product_id()) {
            Some(kind) => {
                res.push(Gopro {
                    kind: kind,
                    serial: info.SerialNumber,
                    device: device,
                })
            },
            None => { continue },
        }


    }
    Ok(res)
}
