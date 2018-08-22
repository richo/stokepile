use std::collections::HashMap;

use failure::Error;

use super::ctx;
use super::ptp_device;

#[derive(Debug)]
pub struct DeviceDescription {
    pub name: String,
}

#[derive(Debug)]
pub enum Device<'a> {
    Gopro(DeviceDescription, ptp_device::Gopro<'a>),
    MassStorage(DeviceDescription),
    Flysight(DeviceDescription),
}

pub fn attached_devices(ctx: &ctx::Ctx) -> Result<Vec<Device>, Error> {
    let mut gopro_serials = HashMap::new();
    // let gopro_serials: HashSet<_> = ctx.cfg.gopros().iter().map(|x| &x.serial).collect();
    for x in ctx.cfg.gopros().iter() {
        gopro_serials.insert(x.serial.clone(), x.name.clone());
    }
    Ok(ptp_device::locate_gopros(ctx)?
       .into_iter()
       .filter_map(|gopro| gopro_serials
                   .get(&gopro.serial)
                   .map(|name| Device::Gopro(DeviceDescription { name: name.to_string() }, gopro)),
       ).collect())
}
