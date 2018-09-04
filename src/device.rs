use std::collections::HashMap;

use failure::Error;
use std::path::Path;

use super::config;
use super::ctx;
use super::ptp_device;
use super::flysight;

use super::staging::Staging;
use super::peripheral::Peripheral;

#[derive(Debug)]
pub struct DeviceDescription {
    pub name: String,
}

#[derive(Debug)]
pub enum Device<'a> {
    Gopro(DeviceDescription, ptp_device::Gopro<'a>),
    MassStorage(DeviceDescription),
    Flysight(DeviceDescription, flysight::Flysight),
}

impl<'a> Device <'a> {
    pub fn stage_files<T>(self, destination: T) -> Result<(), Error>
    where T: AsRef<Path> {
        match self {
            Device::Gopro(desc, gopro) => {
                gopro.stage_files(&desc.name, destination)
            },
            Device::MassStorage(_desc) => {
                unreachable!();
            },
            Device::Flysight(desc, flysight) => {
                flysight.stage_files(&desc.name, destination)
            },
        }
    }
}

pub fn attached_devices(ctx: &ctx::Ctx) -> Result<Vec<Device>, Error> {
    let mut devices = vec![];

    // Should errors actually stop us finding other devices?
    devices.extend(locate_gopros(&ctx)?);
    devices.extend(locate_flysights(&ctx.cfg)?);

    Ok(devices)
}

fn locate_gopros<'a>(ctx: &'a ctx::Ctx) -> Result<impl Iterator<Item = Device<'a>>, Error> {
    let gopro_serials: HashMap<_, _> = ctx.cfg.gopros()
        .iter()
        .map(|x| (x.serial.clone(), x.name.clone()))
        .collect();

    Ok(ptp_device::locate_gopros(ctx)?
       .into_iter()
       .filter_map(move |gopro| gopro_serials
                   .get(&gopro.serial)
                   .map(|name| Device::Gopro(
                           DeviceDescription { name: name.to_string() },
                           gopro))))
}

// TODO(richo) figure out why this doesn't work
// fn locate_flysights<'a, T>(cfg: &config::Config) -> Result<T, Error>
// where T: Iterator<Item = Device<'a>> {
fn locate_flysights<'a>(cfg: &'a config::Config) -> Result<impl Iterator<Item = Device<'a>>, Error> {
    Ok(cfg.flysights()
        .iter()
        .filter_map(|fly_cfg| fly_cfg
                    .flysight()
                    .get()
                    .map(|flysight| Device::Flysight(
                            DeviceDescription { name: fly_cfg.name.clone() },
                            flysight))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::config::Config;

    #[test]
    fn test_locates_flysights() {
        let cfg = Config::from_file("test-data/archiver.toml").unwrap();
        let flysights: Vec<_> = locate_flysights(&cfg).unwrap().collect();
        assert_eq!(flysights.len(), 1);
        if let Device::Flysight(ref desc, ref flysight) = flysights[0] {
            assert_eq!(&flysight.name, "data");
        } else {
            panic!("Unsure what we ended up with: {:?}", flysights);
        }
    }
}
