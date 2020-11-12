#[macro_use]
extern crate log;

use failure::Error;
use lazy_static::lazy_static;

use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Mutex, Arc};
use std::thread;
use std::time;

use stokepile::config;
use stokepile::ctx::Ctx;
use stokepile::device;

type DeviceState<'a> = HashMap<device::Device<'a>, Arc<Mutex<AttachedDeviceState>>>;

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
    /// A device we're not sure about, this was its last state.
    Indeterminate(Box<AttachedDeviceState>),
}

fn poll_devices(state: &mut DeviceState) -> Result<(), Error> {
    // READ ME AND DISPAIR
    // attached_devices() creates a new libusb::connection for each device it finds when
    // translating them into Device objects.
    //
    // The new ones are Drop'd when they're not inserted into the Map so *hopefully* this results
    // in one persistent connection and at most one short lived one at a time, but realistically
    // this probably means the API is defective.
    info!("Attached devices:");
    // First, we go through and set all states to Indeterminate.
    //
    // If another thread preempts this by setting its state to Finished, even though the device is
    // gone, that's ok. The next loop through this method will correct set it to Disconnected.
    for state in state.values() {
        let mut inner = state.lock().expect("Setting indeterminate lock");
        let new_state = AttachedDeviceState::Indeterminate(Box::new(inner.clone()));
        *inner = new_state;
    }

    // Look through all the attached devices, setting their state to Connected if they're new, or
    // unwrapping Indeterminate if that's what we find.
    for device in device::attached_devices(&CTX)? {
        info!("  {:?}", device);
        state.entry(device)
            .and_modify(|v| {
                let mut inner = v.lock().expect("Unwrapping indeterminate lock");
                if let AttachedDeviceState::Indeterminate(inner_state) = inner.deref() {
                    *inner= *inner_state.clone();
                }
            })
            .or_insert_with(|| Arc::new(Mutex::new(AttachedDeviceState::Connected)));
    }

    // Finally, we look through them all, and set anything that's Indeterminate to Disconnected.
    for state in state.values() {
        let mut inner = state.lock().expect("Disconnected lock");
        if let AttachedDeviceState::Indeterminate(_) = inner.deref() {
            *inner = AttachedDeviceState::Disconnected;
        }
    }

    Ok(())
}

fn spawn_worker_threads(state: &mut DeviceState, _work: fn(&device::Device)) {
    for (device, state) in state.iter_mut() {
        let mut inner = state.lock().expect("Worker thread lock");
        if let AttachedDeviceState::Connected = inner.deref() {
            info!("Inspecting {:?}", &device);
            info!("Dispatching worker thread");
            trace!("Current strong count: {}", Arc::strong_count(state));
            *inner = AttachedDeviceState::Processing;


            // Dispatch a thread to handle this device, and update the state when it's done.
            let inner_state = Arc::clone(state);
            thread::spawn(move || {
                info!("Starting worker thread");
                // Urgh, does this mean I also need to wrap the devices in mutexes..
                // work(device);
                info!("Work finished, Marking device as complete");
                *inner_state.lock().expect("Setting complete lock") = AttachedDeviceState::Complete;
            });
        }
    }
}

lazy_static! {
    static ref CTX: Ctx = {
        let cfg = config::Config::from_file("stokepile.toml").expect("Couldn't create config");
        Ctx::create(cfg).expect("Couldn't create config")
    };
}

fn main() {
    stokepile::cli::run(|| {
        trace!("Creating device state");
        let mut state: DeviceState = Default::default();

        poll_devices(&mut state)?;
        spawn_worker_threads(&mut state,
                             |_device| thread::sleep(time::Duration::from_secs(10)));

        // The main polling loop
        loop {
            info!("State: {:?}", &state);
            poll_devices(&mut state)?;
            thread::sleep(time::Duration::from_secs(2));
        }
    })
}

