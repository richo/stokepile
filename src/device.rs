use std::collections::HashSet;

use failure::Error;

use super::ctx;
use super::ptp_device;

pub struct GoproConnection {

}
pub struct MassStorageConnection;
pub struct FlysightConnection;

pub enum Device {
    Gopro(GoproConnection),
    MassStorage(MassStorageConnection),
    Flysight(FlysightConnection),
}

pub fn attached_devices(ctx: &ctx::Ctx) -> Result<Vec<Device>, Error> {
    let mut res = vec![];

    let gopro_serials: HashSet<_> = ctx.cfg.gopros().iter().map(|x| &x.serial).collect();
    for gopro in ptp_device::locate_gopros(ctx)?.iter() {
        if gopro_serials.contains(&gopro.serial) {
            res.push(Device::Gopro(GoproConnection{}))
        }
    }

    Ok(res)
}
