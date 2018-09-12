use std::collections::HashMap;
use std::path::Path;

use failure::Error;

use super::config;
use super::ctx;
use super::flysight;
use super::mass_storage;
use super::peripheral::Peripheral;
use super::ptp_device;
use super::staging::Staging;

#[derive(Debug)]
pub struct DeviceDescription {
    pub name: String,
}

#[derive(Debug)]
pub enum Device<'a> {
    Gopro(DeviceDescription, ptp_device::Gopro<'a>),
    MassStorage(DeviceDescription, mass_storage::MassStorage),
    Flysight(DeviceDescription, flysight::Flysight),
}

impl<'a> Device<'a> {
    pub fn stage_files<T>(self, destination: T) -> Result<(), Error>
    where
        T: AsRef<Path>,
    {
        match self {
            Device::Gopro(desc, gopro) => gopro.stage_files(&desc.name, destination),
            Device::MassStorage(desc, mass_storage) => {
                mass_storage.stage_files(&desc.name, destination)
            }
            Device::Flysight(desc, flysight) => flysight.stage_files(&desc.name, destination),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Device::Gopro(ref desc, _)
            | Device::MassStorage(ref desc, _)
            | Device::Flysight(ref desc, _) => &desc.name[..],
        }
    }
}

pub fn attached_devices(ctx: &ctx::Ctx) -> Result<Vec<Device>, Error> {
    let mut devices = vec![];

    // Should errors actually stop us finding other devices?
    devices.extend(locate_gopros(&ctx)?);
    devices.extend(locate_flysights(&ctx.cfg)?);
    devices.extend(locate_mass_storages(&ctx.cfg)?);

    Ok(devices)
}

fn locate_gopros<'a>(ctx: &'a ctx::Ctx) -> Result<impl Iterator<Item = Device<'a>>, Error> {
    let gopro_serials: HashMap<_, _> = ctx
        .cfg
        .gopros()
        .iter()
        .map(|x| (x.serial.clone(), x.name.clone()))
        .collect();

    Ok(ptp_device::locate_gopros(ctx)?
        .into_iter()
        .filter_map(move |gopro| {
            gopro_serials.get(&gopro.serial).map(|name| {
                Device::Gopro(
                    DeviceDescription {
                        name: name.to_string(),
                    },
                    gopro,
                )
            })
        }))
}

fn locate_flysights<'a>(
    cfg: &'a config::Config,
) -> Result<impl Iterator<Item = Device<'a>>, Error> {
    Ok(cfg.flysights().iter().filter_map(|cfg| {
        cfg.flysight().get().map(|fs| {
            Device::Flysight(
                DeviceDescription {
                    name: cfg.name.clone(),
                },
                fs,
            )
        })
    }))
}

fn locate_mass_storages<'a>(
    cfg: &'a config::Config,
) -> Result<impl Iterator<Item = Device<'a>>, Error> {
    Ok(cfg.mass_storages().iter().filter_map(|cfg| {
        cfg.mass_storage().get().map(|ms| {
            Device::MassStorage(
                DeviceDescription {
                    name: cfg.name.clone(),
                },
                ms,
            )
        })
    }))
}

#[cfg(test)]
mod tests {
    use super::config::Config;
    use super::*;

    #[test]
    fn test_locates_flysights() {
        let cfg = Config::from_file("test-data/archiver.toml").unwrap();
        let flysights: Vec<_> = locate_flysights(&cfg).unwrap().collect();
        assert_eq!(flysights.len(), 1);
        if let Device::Flysight(ref _desc, ref flysight) = flysights[0] {
            assert_eq!(&flysight.name()[..], "data");
        } else {
            panic!("Unsure what we ended up with: {:?}", flysights);
        }
    }

    #[test]
    fn test_locates_mass_storages() {
        let cfg = Config::from_file("test-data/archiver.toml").unwrap();
        let flysights: Vec<_> = locate_flysights(&cfg).unwrap().collect();
        assert_eq!(flysights.len(), 1);
        if let Device::Flysight(ref desc, ref flysight) = flysights[0] {
            assert_eq!(&flysight.name()[..], "data");
        } else {
            panic!("Unsure what we ended up with: {:?}", flysights);
        }
    }
}
