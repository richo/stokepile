use std::fmt;

use failure::Error;
use libusb;

// This is used, I guess maybe I should make it a phantomdata or whatever?
#[allow(unused_imports)]
use lockfile::Lockfile;

use crate::config;
use crate::mailer;
use crate::pushover_notifier;
use crate::staging::StagingDirectory;

/// Ctx is the global context object. Constructed by consuming a `config::Config`.
pub struct Ctx {
    /// a USB context, used for finding and interacting with PTP devices
    pub usb_ctx: libusb::Context,
    pub cfg: config::Config,
    /// An optional notifier to call on changes to uploads.
    pub notifier: Option<pushover_notifier::PushoverNotifier>,
    /// An optional mailer that will be used to send reports when uploads finish or fail.
    pub mailer: Option<mailer::SendgridMailer>,
    /// The staging adaptor that we'll be using.
    pub staging: StagingDirectory,
    // This lock is optional, since we can opt into building it without, but by making the lock
    // part of this API we can't accidentally end up not having one.
    _lock: Option<lockfile::Lockfile>,
}

impl fmt::Debug for Ctx {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Ctx")
            .field("usb_ctx", &"libusb::Context { ... }")
            .field("cfg", &self.cfg)
            .field("staging", &self.staging)
            .field("notifier", &self.notifier)
            .field("mailer", &self.mailer)
            .finish()
    }
}

fn acquire_lock() -> Result<lockfile::Lockfile, Error> {
    let home = config::get_home()?;
    let lock_path = home.as_ref().join(".archiver.lock");
    info!("Acquiring the archiver lock at {:?}", &lock_path);
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

    /// Create a new context object without acquiring the archiver lock.
    ///
    /// Holding an unlocked Ctx allows you to perform destructive operations with no
    /// synchronisation, it is the consumers responsibility to ensure this does not occur.
    pub fn create_without_lock(cfg: config::Config) -> Result<Ctx, Error> {
        Self::create_ctx(cfg, false)
    }

    fn create_ctx(cfg: config::Config, should_lock: bool) -> Result<Ctx, Error> {
        let notifier = cfg.notifier();
        let mailer = cfg.mailer();
        let staging = cfg.staging()?;

        let _lock = if should_lock {
            Some(acquire_lock()?)
        } else {
            None
        };

        Ok(Ctx {
            usb_ctx: libusb::Context::new()?,
            cfg,
            notifier,
            mailer,
            staging,
            _lock,
        })
    }

    pub fn staging(&self) -> &StagingDirectory {
        &self.staging
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
