extern crate libusb;
extern crate ptp;

use std::fmt;
use std::cmp;
use std::io::{self, Read};

use failure::Error;

use super::ctx;

#[derive(Debug)]
pub struct GoproFile {
    pub capturedate: String,
    pub filename: String,
    // TODO(richo) I think this handle gets invalidated when we close the session down
    handle: u32,
    size: u32,
}

impl GoproFile {
    pub fn reader<'a, 'b>(self, conn: &'a mut GoproConnection<'b>) -> GoproFileReader<'a, 'b> {
        GoproFileReader {
            conn: conn,
            handle: self.handle,
            offset: 0,
            size: self.size,
        }
    }
}

pub struct GoproFileReader<'a, 'b: 'a> {
    conn: &'a mut GoproConnection<'b>,
    handle: u32,
    offset: u32,
    size: u32,
}

impl<'a, 'b> Read for GoproFileReader<'a, 'b> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // Tragically, ptp really wants to allocate it's own memory :(
        // If I have luck with my other patches, we can try to upstream something
        let mut size = buf.len();
        if self.offset + size as u32 > self.size {
            size = (self.size - self.offset) as usize;
        }
        let vec = self.conn.camera.get_partialobject(self.handle,
                                                     self.offset,
                                                     size as u32,
                                                     None).expect("FIXME couldn't read from buf");
        println!("Got {} bytes from the camera", vec.len());
        buf.copy_from_slice(&vec[..]);
        println!("Telling the consumer we read {}", vec.len());
        self.offset += vec.len() as u32;
        return Ok(vec.len());
    }
}

#[repr(u16)]
#[derive(Eq,PartialEq,Debug)]
enum GoproObjectFormat {
    Directory = 0x3001,
    Video = 0x300d,
    GetStarted = 0x3005,
}

impl GoproObjectFormat {
    fn from_u16(format: u16) -> Option<GoproObjectFormat> {
        use self::GoproObjectFormat::*;
        match format {
            0x3001 => Some(Directory),
            0x300d => Some(Video),
            0x3005 => Some(GetStarted),
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
    Hero5Black,
    UnknownGopro,
}

impl GoproKind {
    fn from_u16(ty: u16) -> Option<GoproKind> {
        use self::GoproKind::*;
        match ty {
            0x000d => Some(Hero4Silver),
            0x002d => Some(Hero5Black),
            _ => Some(UnknownGopro),
        }
    }
}

// Specialising to PTP devices later might be neat, but for now this is fine
pub struct Gopro<'a> {
    pub kind: GoproKind,
    pub serial: String,
    device: libusb::Device<'a>,
}

pub struct GoproConnection<'a> {
    gopro: Gopro<'a>,
    camera: ptp::PtpCamera<'a>,
}

impl<'a> GoproConnection<'a> {
    pub fn files(&mut self) -> Result<Vec<GoproFile>, Error> {
        let mut out = vec![];
        let timeout = None;

        // TODO(richo) Encapsulate this into some object that actually lets you poke around in the
        // libusb::Device and won't let you not close your session, etc.
        let filehandles = self.camera.get_objecthandles_all(0xFFFFFFFF,
                                                            Some(GoproObjectFormat::Video as u32),
                                                            timeout)?;
        for filehandle in filehandles {
            let object = self.camera.get_objectinfo(filehandle, timeout)?;
            assert_eq!(GoproObjectFormat::from_u16(object.ObjectFormat),
                       Some(GoproObjectFormat::Video));
            out.push(GoproFile {
                capturedate: object.CaptureDate,
                filename: object.Filename,
                handle: filehandle,
                size: object.ObjectCompressedSize,
            })
        }

        Ok(out)
    }
}

impl<'a> Drop for GoproConnection<'a> {
    fn drop(&mut self) {
        // If this fails.. who cares I guess
        let _ = self.camera.close_session(None);
    }
}

impl<'a> Gopro<'a> {
    pub fn new(kind: GoproKind, serial: String, device: libusb::Device) -> Result<Gopro, Error> {
        Ok(Gopro {
            kind: kind,
            serial: serial,
            device: device,
        })
    }

    pub fn connect(self) -> Result<GoproConnection<'a>, Error> {
        let mut camera = ptp::PtpCamera::new(&self.device)?;
        camera.open_session(None)?;

        Ok(GoproConnection {
            gopro: self,
            camera: camera,
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

impl<'a> fmt::Debug for GoproConnection<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        self.gopro.fmt(fmt)
    }
}

pub fn locate_gopros(ctx: &ctx::Ctx) -> Result<Vec<Gopro>, Error> {
    let mut res = vec![];

    for mut device in ctx.usb_ctx.devices()?.iter() {
        let device_desc = device.device_descriptor()?;

        if device_desc.vendor_id() != GOPRO_VENDOR {
            continue
        }

        // We'll just use the Manufacturer tag in the PtpDevice

        let mut camera = ptp::PtpCamera::new(&device)?;
        let info = camera.get_device_info(None)?;

        if info.Manufacturer != GOPRO_MANUFACTURER {
            continue
        }

        // TODO(richo) include the product from info so we can do something useful with unknowns
        match GoproKind::from_u16(device_desc.product_id()) {
            Some(kind) => {
                res.push(Gopro::new(kind, info.SerialNumber, device)?);
            },
            None => { continue },
        }


    }
    Ok(res)
}
