use std::fmt;
use std::fs;
use std::io;
use std::path::PathBuf;

use failure::Error;
use libusb;

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
}

impl fmt::Debug for Ctx {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
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
        let staging = create_or_find_staging(&cfg)?;
        // TODO(richo) offload figuring out what notifier we should use to the config
        let notifier = cfg.notifier();
        let mailer = cfg.mailer();

        Ok(Ctx {
            usb_ctx: libusb::Context::new()?,
            cfg,
            staging,
            notifier,
            mailer,
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
