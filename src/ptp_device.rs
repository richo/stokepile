extern crate libusb;
extern crate ptp;
extern crate chrono;
extern crate sha2;
extern crate hashing_copy;
extern crate serde_json;

use std::path::Path;
use chrono::prelude::*;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read};

use failure::Error;

use super::ctx;
use super::staging::{UploadDescriptor, Staging};

fn parse_gopro_date(date: &str) -> Result<DateTime<Local>, chrono::ParseError> {
    Local.datetime_from_str(date, "%Y%m%dT%H%M%S")
}

#[derive(Debug)]
pub struct GoproFile {
    pub capturedate: String,
    pub filename: String,
    // TODO(richo) I think this handle gets invalidated when we close the session down
    handle: u32,
    size: u32,
}

impl GoproFile {
    pub fn reader<'a, 'b>(&'a self, conn: &'a mut GoproConnection<'b>) -> GoproFileReader<'a, 'b> {
        GoproFileReader {
            conn: conn,
            handle: self.handle,
            offset: 0,
            size: self.size,
        }
    }

    pub fn delete<'a, 'b>(self, conn: &'a mut GoproConnection<'b>) -> Result<(), Error> {
        // lol how even does into
        let ret = conn.camera.delete_object(self.handle, None)?;
        Ok(ret)
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
        &buf[..vec.len()].copy_from_slice(&vec[..]);
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
    // TODO(richo) having a name in here would simplify the Staging impl
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

    pub fn power_down(&mut self) -> Result<(), ptp::Error> {
        self.camera.power_down(None)
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

impl<'a> Staging for Gopro<'a> {
    // Consumes self, purely because connect does
    fn stage_files<T>(self, name: &str, destination: T) -> Result<(), Error>
    where T: AsRef<Path> {
        let mut plan = Vec::new();
        let mut conn = self.connect()?;

        // We build a vector first because we need to use the connection again to read them, and
        // the connection is borrowed by the files iterator
        for file in conn.files()? {
            let capture_time = parse_gopro_date(&file.capturedate)?;
            let size = file.size as u64;
            plan.push((file, UploadDescriptor {
                capture_time,
                device_name: name.to_string(),
                // TODO(richo) is this always true?
                extension: "mp4".to_string(),
                sha2: [0; 32],
                size,
            }));
        }

        for (file, mut desc) in plan {
            let staging_name = desc.staging_name();
            let manifest_name = desc.manifest_name();

            let mut options = fs::OpenOptions::new();
            let options = options.write(true).create_new(true);

            let staging_path = destination.as_ref().join(&staging_name);
            let manifest_path = destination.as_ref().join(&manifest_name);

            info!("Staging {}", &staging_name);
            trace!(" To {:?}", staging_path);
            {
                let mut staged = options.open(&staging_path)?;
                let (size, hash) = hashing_copy::copy_and_hash::<_, _, sha2::Sha256>(&mut file.reader(&mut conn), &mut staged)?;
                assert_eq!(size, desc.size);
                info!("Shasum: {:x}", hash);
                info!("size: {:x}", size);
                desc.sha2.copy_from_slice(&hash);
            } // Ensure that we've closed our staging file

            {
                info!("Manifesting {}", &manifest_name);
                trace!(" To {:?}", manifest_path);
                let mut staged = options.open(&manifest_path)?;
                serde_json::to_writer(&mut staged, &desc)?;
            }

            // Once I'm more confident that I haven't fucked up staging
            // file.delete()
        }

        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parses_gopro_date_correctly() {
        let dt = Local.ymd(2015, 1, 1).and_hms(0, 6, 49);
        // TODO(richo) get better testcases
        assert_eq!(parse_gopro_date("20150101T000649"),
                   Ok(dt.clone()));
    }
}
