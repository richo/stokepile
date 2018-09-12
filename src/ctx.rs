use std::path::PathBuf;

use libusb;

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
