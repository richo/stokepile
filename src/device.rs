use std::collections::HashMap;

use failure::Error;

use crate::config;
use crate::ctx;
use crate::ptp_device;
use crate::staging::{Staging, StageableLocation, Stager};
use crate::mountable::{Mountable, MountableFilesystem};

#[derive(Eq, PartialEq, Debug, Hash)]
pub struct DeviceDescription {
    pub name: String,
}

#[derive(Eq, PartialEq, Debug, Hash)]
// TODO(richo) if we implement the ptp connection stuff in terms of mount, suddenly we can unify
// this whole thing behind a trait!, and I think the DeviceDescription is now pointless as well.
pub enum Device<'a> {
    Gopro(DeviceDescription, ptp_device::Gopro<'a>),
    MassStorage(DeviceDescription, config::MassStorageConfig),
    Flysight(DeviceDescription, config::FlysightConfig),
}

impl Device<'_> {
    pub fn stage_files<T: StageableLocation>(self, stager: &Stager<T>) -> Result<usize, Error> {
        match self {
            Device::Gopro(desc, gopro) => {
                Mountable::mount(gopro)?.stage_files(&desc.name, stager)
            },
            Device::MassStorage(desc, mass_storage) => {
                Mountable::mount(mass_storage)?.stage_files(&desc.name, stager)
            },
            Device::Flysight(desc, flysight) => {
                Mountable::mount(flysight)?.stage_files(&desc.name, stager)
            },
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

pub fn attached_devices(ctx: &ctx::Ctx) -> Result<Vec<Device<'_>>, Error> {
    let mut devices = vec![];

    // Should errors actually stop us finding other devices?
    devices.extend(locate_gopros(&ctx)?);
    devices.extend(locate_flysights(&ctx.cfg)?);
    devices.extend(locate_mass_storages(&ctx.cfg)?);

    Ok(devices)
}

fn locate_gopros(ctx: &ctx::Ctx) -> Result<impl Iterator<Item = Device<'_>>, Error> {
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

fn locate_flysights(
    cfg: &config::Config,
) -> Result<impl Iterator<Item = Device<'_>>, Error> {
    Ok(cfg.flysights().iter().filter_map(|cfg| {
        cfg.clone().get().map(|fs| {
            Device::Flysight(
                DeviceDescription {
                    name: cfg.name().to_string(),
                },
                fs,
            )
        })
    }))
}

fn locate_mass_storages(
    cfg: &config::Config,
) -> Result<impl Iterator<Item = Device<'_>>, Error> {
    Ok(cfg.mass_storages().iter().filter_map(|cfg| {
        cfg.clone().get().map(|ms| {
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
    use crate::config::Config;
    use super::*;

    #[test]
    fn test_locates_flysights() {
        let cfg = Config::from_file("test-data/stokepile.toml").unwrap();
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
        let cfg = Config::from_file("test-data/stokepile.toml").unwrap();
        let flysights: Vec<_> = locate_flysights(&cfg).unwrap().collect();
        assert_eq!(flysights.len(), 1);
        if let Device::Flysight(ref _desc, ref flysight) = flysights[0] {
            assert_eq!(&flysight.name()[..], "data");
        } else {
            panic!("Unsure what we ended up with: {:?}", flysights);
        }
    }
}
