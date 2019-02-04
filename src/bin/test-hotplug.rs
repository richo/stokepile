#[macro_use]
extern crate log;

use libusb;
use failure::Error;
use lazy_static::lazy_static;

use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Mutex, Arc};
use std::thread;
use std::time;

use archiver::config::{self, DeviceConfig};
use archiver::ctx::Ctx;
use archiver::device;

/// A device that either is attached, or previously has been, to this uploader session.
#[derive(Debug, Clone, Eq, PartialEq)]
enum AttachedDeviceState {
    /// A device that is currently attached, but has not yet been processed.
    Connected,
    /// A device that is currently being processed.
    Processing,
    /// A device that has been processed, and is still attached.
    Complete,
    /// A device that was disconnected from this session.
    Disconnected,
}

fn dispatch_gopro_watcher(config: &config::GoproConfig) {
    info!("Dispatching a thread to monitor {:?}", config);
}

// TODO(richo) lift this into archiver::cli? Or too magic?
lazy_static! {
    static ref ctx: Ctx = {
        let cfg = config::Config::from_file("archiver.toml").expect("Couldn't create config");
        Ctx::create(cfg).expect("Couldn't create config")
    };
}

fn main() {
    archiver::cli::run(|| {
        info!("Fetching configured devices");
        let devices = ctx.cfg.configured_devices();

        for device in devices {
            // Prototyping with gopros for now
            if let DeviceConfig::Gopro(gopro_cfg) = device {
                dispatch_gopro_watcher(gopro_cfg);
            } else {
                info!("Ignoring device {:?} for now", &device);
            }
        }

        Ok(())
    })
}

