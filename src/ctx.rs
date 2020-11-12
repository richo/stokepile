use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use failure::Error;

#[cfg(feature = "usb")]
use libusb;
#[cfg(not(feature = "usb"))]
use crate::dummy_libusb as libusb;


// This is used, I guess maybe I should make it a phantomdata or whatever?
#[allow(unused_imports)]
use lockfile::Lockfile;

use crate::config;
use crate::mailer;
use crate::pushover_notifier::Notify;

/// Ctx is the global context object. Constructed by consuming a `config::Config`.
pub struct Ctx {
    /// a USB context, used for finding and interacting with PTP devices
    pub usb_ctx: libusb::Context,
    pub cfg: config::Config,
    /// An optional notifier to call on changes to uploads.
    notifier: Option<Box<dyn Notify>>,
    /// An optional mailer that will be used to send reports when uploads finish or fail.
    pub mailer: Option<mailer::SendgridMailer>,
    // This lock is optional, since we can opt into building it without, but by making the lock
    // part of this API we can't accidentally end up not having one.
    _lock: Option<lockfile::Lockfile>,
    /// Whether or not we ought to proceed, this provides a hook for early exit
    running: Arc<AtomicBool>,
}

impl fmt::Debug for Ctx {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Ctx")
            .field("usb_ctx", &"libusb::Context { ... }")
            .field("cfg", &self.cfg)
            .field("notifier", &self.notifier)
            .field("mailer", &self.mailer)
            .finish()
    }
}

fn acquire_lock() -> Result<lockfile::Lockfile, Error> {
    let home = config::get_home()?;
    let lock_path = home.as_ref().join(".stokepile.lock");
    info!("Acquiring the stokepile lock at {:?}", &lock_path);
    Ok(lockfile::Lockfile::create(lock_path)?)
}

impl Ctx {
    /// Create a new context object.
    ///
    /// This method has many side effects, creating a libusb context, creating the staging
    /// direectory if it does not exist, etc.
    pub fn create(cfg: config::Config) -> Result<Ctx, Error> {
        Self::create_ctx(cfg, true)
    }

    /// Create a new context object without acquiring the stokepile lock.
    ///
    /// Holding an unlocked Ctx allows you to perform destructive operations with no
    /// synchronisation, it is the consumers responsibility to ensure this does not occur.
    pub fn create_without_lock(cfg: config::Config) -> Result<Ctx, Error> {
        Self::create_ctx(cfg, false)
    }

    fn create_ctx(cfg: config::Config, should_lock: bool) -> Result<Ctx, Error> {
        let notifier = cfg.notifier();
        let mailer = cfg.mailer();

        let _lock = if should_lock {
            Some(acquire_lock()?)
        } else {
            None
        };

        let running = Arc::new(AtomicBool::new(true));

        Ok(Ctx {
            usb_ctx: libusb::Context::new()?,
            cfg,
            notifier,
            mailer,
            _lock,
            running,
        })
    }

    // TODO(richo) We should be able to make this info
    // &impl MountablePeripheral<Output=MountedStaging>
    // at some point
    pub fn staging(&self) -> config::StagingConfig {
        self.cfg.staging()
    }

    pub fn notify(&self, msg: &str) -> Result<(), Error> {
        if let Some(notifier) = &self.notifier {
            return notifier.notify(msg)
        }
        Ok(())
    }

    pub fn setup_ctrlc_handler(&self) -> Result<(), ctrlc::Error> {
        let running = self.running.clone();
        ctrlc::set_handler(move || {
            running.store(false, Ordering::SeqCst);
        })
    }

    pub fn running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locks_actually_lock() {
        let lock = acquire_lock();
        assert!(lock.is_ok());
        let another_lock = acquire_lock();
        assert!(another_lock.is_err());
    }
}
