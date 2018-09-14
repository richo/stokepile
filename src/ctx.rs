use std::path::PathBuf;
use std::fs;
use std::io;

use libusb;
use failure::Error;

use config;
use mailer;
use pushover_notifier;

pub struct Ctx {
    pub usb_ctx: libusb::Context,
    pub cfg: config::Config,
    pub staging: PathBuf,
    pub notifier: Option<pushover_notifier::PushoverNotifier>,
    pub mailer: Option<mailer::SendgridMailer>,
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
    let path = cfg
        .staging_dir()?
        .unwrap_or_else(|| {
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
