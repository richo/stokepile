extern crate libusb;
extern crate ptp;

use std::fs::File;
use std::io::prelude::*;
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
    pub device: libusb::Device<'a>,
}

impl<'a> Gopro<'a> {
    // TODO(richo) This should be an iterator
    // The GoproFile (and friends) can implement a Materialise trait, which should at least make
    // sure I can only have a single file in memory. Once I figure out what all the upload
    // endpoints look like we can figure out how to make sure streaming works
    pub fn files(&self) -> Result<Vec<GoproFile>, Error> {
        let mut out = vec![];
        let timeout = None;

        let mut camera = ptp::PtpCamera::new(&self.device)?;

        // TODO(richo) Encapsulate this into some object that actually lets you poke around in the
        // libusb::Device and won't let you not close your session, etc.
        camera.open_session(timeout);
        for storage_id in camera.get_storageids(timeout)? {
            // let storage_info = camera.get_storage_info(storage_id, timeout);
            // println!("storage_info: {:?}", storage_info);

            for handle in camera.get_objecthandles_all(storage_id, None, timeout)? {
                for innerhandle in camera.get_objecthandles(storage_id, handle, None, timeout)? {
                    // println!("innerhandle: {:?}", innerhandle);
                    let object = camera.get_objectinfo(innerhandle, timeout)?;
                    println!("object: {:?}", object);
                    match GoproObjectFormat::from_u16(object.ObjectFormat) {
                        Some(GoproObjectFormat::Directory) => {},
                        Some(GoproObjectFormat::File) => { self.handle_file(object, handle, &mut out) },
                        _ => {},
                    }
                }
            }
        }
        camera.close_session(timeout);

        Ok(out)
    }

    fn handle_file(&self, file: ptp::PtpObjectInfo, handle: u32, out: &mut Vec<GoproFile>) {
// PtpObjectInfo { StorageID: 65537, ObjectFormat: 12301, ProtectionStatus: 0, ObjectCompressedSize: 14853083, ThumbFormat: 14337, ThumbCompressedSize: 14233, ThumbPixWidth: 0, ThumbPixHeight: 0, ImagePixWidth: 1920, ImagePixHeight: 1080, ImageBitDepth: 0, ParentObject: 2, AssociationType: 1, AssociationDesc: 0, SequenceNumber: 0, Filename: "GOPR9833.MP4", CaptureDate: "20150101T000649", ModificationDate: "", Keywords: "" }
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
