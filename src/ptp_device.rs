use std::fmt;
use std::io::{self, Read};
use std::rc::Rc;
use std::sync::Mutex;

use crate::ctx;
use crate::staging::{StageFromDevice, DateTimeUploadable};
use crate::mountable::{Mountable};

use chrono;
use chrono::prelude::*;
use failure::{Error, ResultExt};

#[cfg(feature = "usb")]
use libusb;
#[cfg(not(feature = "usb"))]
use crate::dummy_libusb as libusb;

#[cfg(feature = "usb")]
use ptp;
#[cfg(not(feature = "usb"))]
use crate::dummy_ptp as ptp;

use std::hash::{Hash, Hasher};

fn parse_gopro_date(date: &str) -> Result<DateTime<Local>, chrono::ParseError> {
    Local.datetime_from_str(date, "%Y%m%dT%H%M%S")
}

pub struct GoproFile<'c> {
    pub capturedate: String,
    // TODO(richo) I think this handle gets invalidated when we close the session down
    handle: u32,
    offset: u32,
    size: u32,
    camera: Rc<Mutex<ptp::PtpCamera<'c>>>,
}

impl<'c> fmt::Debug for GoproFile<'c> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("GoproFile")
            .field("handle", &self.handle)
            .field("offset", &self.offset)
            .field("size", &self.size)
            .field("camera", &"Rc<Mutex<ptp::PtpCamera<'c> { ... }>>")
            .finish()
    }
}

impl<'c> DateTimeUploadable for GoproFile<'c> {
    type Reader = GoproFile<'c>;

    fn extension(&self) -> &str {
        "mp4"
    }

    fn capture_datetime(&self) -> Result<DateTime<Local>, chrono::ParseError> {
        parse_gopro_date(&self.capturedate)
    }

    fn reader(&mut self) -> &mut GoproFile<'c> {
        self
    }

    fn delete(&mut self) -> Result<(), Error> {
        // lol how even does into
        Ok(self
            .camera
            .lock()
            .unwrap()
            .delete_object(self.handle, None)?)
    }

    fn size(&self) -> Result<u64, Error> {
        Ok(u64::from(self.size))
    }
}

impl<'b> Read for GoproFile<'b> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // Tragically, ptp really wants to allocate it's own memory :(
        // If I have luck with my other patches, we can try to upstream something
        let mut size = buf.len();
        if self.offset + size as u32 > self.size {
            size = (self.size - self.offset) as usize;
        }
        let vec = self
            .camera
            .lock()
            .unwrap()
            .get_partialobject(self.handle, self.offset, size as u32, None)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        buf[..vec.len()].copy_from_slice(&vec[..]);
        self.offset += vec.len() as u32;
        Ok(vec.len())
    }
}

#[repr(u16)]
#[derive(Eq, PartialEq, Debug)]
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
const GOPRO_MANUFACTURER: &str = "GoPro";

#[repr(u16)]
#[derive(Eq, PartialEq, Debug, Hash)]
pub enum GoproKind {
    Hero4Silver,
    Hero4Black,
    Hero4Session,
    Hero2018,
    Hero5Black,
    Hero5Session,
    Hero8Black,
    UnknownGopro(u16),
}

impl GoproKind {
    fn from_u16(ty: u16) -> Option<GoproKind> {
        use self::GoproKind::*;
        match ty {
            0x000d => Some(Hero4Silver),
            0x000e => Some(Hero4Black),
            0x000f => Some(Hero4Session),
            0x002d => Some(Hero2018),
            0x0027 => Some(Hero5Black),
            0x0029 => Some(Hero5Session),
            0x0049 => Some(Hero8Black),
            _ => Some(UnknownGopro(ty)),
        }
    }
}

// Specialising to PTP devices later might be neat, but for now this is fine
pub struct Gopro<'d> {
    // TODO(richo) having a name in here would simplify the Staging impl
    pub kind: GoproKind,
    pub serial: String,
    device: libusb::Device<'d>,
}

impl<'d> PartialEq for Gopro<'d> {
    fn eq(&self, other: &Gopro<'d>) -> bool {
        self.kind == other.kind &&
            self.serial == other.serial
    }
}

impl<'d> Eq for Gopro<'d> {}

impl<'d> Hash for Gopro<'d> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
        self.serial.hash(state);
    }
}

pub struct GoproConnection<'c> {
    gopro: Gopro<'c>,
    camera: Rc<Mutex<ptp::PtpCamera<'c>>>,
}

impl<'c> StageFromDevice for GoproConnection<'c> where {
    type FileType = GoproFile<'c>;

    fn files(&self) -> Result<Vec<GoproFile<'c>>, Error> {
        let mut out = vec![];
        let timeout = None;

        // TODO(richo) Encapsulate this into some object that actually lets you poke around in the
        // libusb::Device and won't let you not close your session, etc.
        let filehandles = self.camera.lock().unwrap().get_objecthandles_all(
            0xFFFF_FFFF,
            Some(GoproObjectFormat::Video as u32),
            timeout,
        )?;
        for filehandle in filehandles {
            let object = self
                .camera
                .lock()
                .unwrap()
                .get_objectinfo(filehandle, timeout)?;
            assert_eq!(
                GoproObjectFormat::from_u16(object.ObjectFormat),
                Some(GoproObjectFormat::Video)
            );
            let file = GoproFile {
                capturedate: object.CaptureDate,
                handle: filehandle,
                offset: 0,
                size: object.ObjectCompressedSize,
                camera: Rc::clone(&self.camera),
            };
            trace!("Adding {:?} to the plan", &file);
            out.push(file)
        }

        info!(
            "Loaded {} files from {:?} serial {}",
            out.len(),
            &self.gopro.kind,
            &self.gopro.serial
        );

        Ok(out)
    }
}

impl<'c> GoproConnection<'c> {
    pub fn power_down(&mut self) -> Result<(), ptp::Error> {
        self.camera.lock().unwrap().power_down(None)
    }
}

impl<'c> Drop for GoproConnection<'c> {
    fn drop(&mut self) {
        // If this fails.. who cares I guess
        info!("Closing session on {:?}", &self);
        let _ = self.camera.lock().unwrap().close_session(None);
    }
}

impl<'a> Mountable for Gopro<'a> {
    type Target = GoproConnection<'a>;

    #[cfg(not(feature = "usb"))]
    fn mount(self) -> Result<GoproConnection<'a>, Error> {
        unimplemented!("Can't mount nonexistant gopros");
    }

    #[cfg(feature = "usb")]
    fn mount(self) -> Result<GoproConnection<'a>, Error> {
        let mut camera = ptp::PtpCamera::new(&self.device)?;
        camera.open_session(None)
            .context("Creating session on camera")?;

        Ok(GoproConnection {
            gopro: self,
            camera: Rc::new(Mutex::new(camera)),
        })
    }
}

impl<'a> Gopro<'a> {
    pub fn new(kind: GoproKind, serial: String, device: libusb::Device<'_>) -> Result<Gopro<'_>, Error> {
        Ok(Gopro {
            kind,
            serial,
            device,
        })
    }
}

impl<'a> fmt::Debug for Gopro<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Gopro")
            .field("kind", &self.kind)
            .field("serial", &self.serial)
            .field("device", &"libusb::Device")
            .finish()
    }
}

impl<'c> fmt::Debug for GoproConnection<'c> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.gopro.fmt(fmt)
    }
}

pub fn locate_gopros(ctx: &ctx::Ctx) -> Result<Vec<Gopro<'_>>, Error> {
    let mut res = vec![];

    // TODO(richo) It'd be really nice to have this still build, but the ptpCamera stuff looks like
    // ti's gunna be rough af.
    #[cfg(feature = "usb")]
    for device in ctx.usb_ctx.devices()?.iter() {
        let device_desc = device.device_descriptor()?;

        if device_desc.vendor_id() != GOPRO_VENDOR {
            continue;
        }

        // We'll just use the Manufacturer tag in the PtpDevice

        let mut camera = ptp::PtpCamera::new(&device)?;
        let info = camera.get_device_info(None)?;

        if info.Manufacturer != GOPRO_MANUFACTURER {
            continue;
        }

        // TODO(richo) include the product from info so we can do something useful with unknowns
        match GoproKind::from_u16(device_desc.product_id()) {
            Some(kind) => {
                res.push(Gopro::new(kind, info.SerialNumber, device)?);
            }
            None => continue,
        }
    }
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parses_gopro_date_correctly() {
        let dt = Local.ymd(2015, 1, 1).and_hms(0, 6, 49);
        // TODO(richo) get better testcases
        assert_eq!(parse_gopro_date("20150101T000649"), Ok(dt.clone()));
    }
}
