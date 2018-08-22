extern crate libusb;
extern crate ptp;

use std::fmt;

use failure::Error;

use super::ctx;

#[derive(Debug)]
pub struct GoproFile {
    capturedate: String,
    filename: String,
    // TODO(richo) I think this handle gets invalidated when we close the session down
    handle: u32,
}

#[repr(u16)]
enum GoproObjectFormat {
    Directory = 0x3001,
    File = 0x300d,
}

impl GoproObjectFormat {
    fn from_u16(format: u16) -> Option<GoproObjectFormat> {
        use self::GoproObjectFormat::*;
        match format {
            0x3001 => Some(Directory),
            0x300d => Some(File),
            _ => None,
        }
    }
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
    device: libusb::Device<'a>,
    camera: ptp::PtpCamera<'a>,
}

impl<'a> Drop for Gopro<'a> {
    fn drop(&mut self) {
        // If this fails.. who cares I guess
        self.camera.close_session(None);
    }
}

impl<'a> Gopro<'a> {
    pub fn new(kind: GoproKind, serial: String, device: libusb::Device) -> Result<Gopro, Error> {
        let mut camera = ptp::PtpCamera::new(&device)?;
        camera.open_session(None)?;
        Ok(Gopro {
            kind: kind,
            serial: serial,
            device: device,
            camera: camera,
        })
    }
    // TODO(richo) This should be an iterator
    // The GoproFile (and friends) can implement a Materialise trait, which should at least make
    // sure I can only have a single file in memory. Once I figure out what all the upload
    // endpoints look like we can figure out how to make sure streaming works
    pub fn files(&mut self) -> Result<Vec<GoproFile>, Error> {
        let mut out = vec![];
        let timeout = None;

        // TODO(richo) Encapsulate this into some object that actually lets you poke around in the
        // libusb::Device and won't let you not close your session, etc.
        for storage_id in self.camera.get_storageids(timeout)? {
            // let storage_info = self.camera.get_storage_info(storage_id, timeout);
            // println!("storage_info: {:?}", storage_info);

            for handle in self.camera.get_objecthandles_all(storage_id, None, timeout)? {
                for innerhandle in self.camera.get_objecthandles(storage_id, handle, None, timeout)? {
                    // println!("innerhandle: {:?}", innerhandle);
                    let object = self.camera.get_objectinfo(innerhandle, timeout)?;
                    println!("object: {:?}", object);
                    match GoproObjectFormat::from_u16(object.ObjectFormat) {
                        Some(GoproObjectFormat::Directory) => {},
                        Some(GoproObjectFormat::File) => { self.handle_file(object, handle, &mut out) },
                        _ => {},
                    }
                }
            }
        }

        Ok(out)
    }

    fn handle_file(&self, file: ptp::PtpObjectInfo, handle: u32, out: &mut Vec<GoproFile>) {
        out.push(GoproFile {
            capturedate: file.CaptureDate,
            filename: file.Filename,
            handle: handle,
        })
    }
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
                res.push(Gopro::new(kind, info.SerialNumber, device)?);
            },
            None => { continue },
        }


    }
    Ok(res)
}
