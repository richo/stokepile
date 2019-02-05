use std::fmt;
use std::fs;
use std::io;
use std::path::PathBuf;

use dirs;
use failure::Error;
use libusb;

// This is used, I guess maybe I should make it a phantomdata or whatever?
#[allow(unused_imports)]
use lockfile::Lockfile;

use crate::config;
use crate::mailer;
use crate::pushover_notifier;

/// Ctx is the global context object. Constructed by consuming a `config::Config`.
pub struct Ctx {
    /// a USB context, used for finding and interacting with PTP devices
    pub usb_ctx: libusb::Context,
    pub cfg: config::Config,
    /// The directory that will be used for staging files before they're uploaded.
    ///
    /// This directory will be treated as durable! Do not set it to `/tmp` if you care about your
    /// files.
    pub staging: PathBuf,
    /// An optional notifier to call on changes to uploads.
    pub notifier: Option<pushover_notifier::PushoverNotifier>,
    /// An optional mailer that will be used to send reports when uploads finish or fail.
    pub mailer: Option<mailer::SendgridMailer>,
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
        let staging = create_or_find_staging(&cfg)?;
        // TODO(richo) offload figuring out what notifier we should use to the config
        let notifier = cfg.notifier();
        let mailer = cfg.mailer();

        let _lock = if should_lock {
            // TODO(richo) for now we just stash this in the user's home directory.
            let home = dirs::home_dir().ok_or(format_err!("Couldn't open HOME"))?;
            let lock_path = home.join(".archiver.lock");
            info!("Acquiring the archiver lock at {:?}", &lock_path);
            Some(lockfile::Lockfile::create(lock_path)?)
        } else {
            None
        };

        Ok(Ctx {
            usb_ctx: libusb::Context::new()?,
            cfg,
            staging,
            notifier,
            mailer,
            _lock,
        })
    }
}

fn create_or_find_staging(cfg: &config::Config) -> Result<PathBuf, Error> {
    let path = cfg.staging_dir()?.unwrap_or_else(|| {
        info!("Staging dir not specified, falling back to `staging`");
        PathBuf::from("staging")
    });

    if let Err(e) = fs::create_dir(&path) {
        if e.kind() == io::ErrorKind::AlreadyExists {
            info!("Reusing existing staging dir");
        } else {
            error!("{:?}", e);
            panic!();
        }
    }

    Ok(path)
}
