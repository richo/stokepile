use failure::Error;
use std::fmt;

use crate::pushover::Pushover;

pub struct PushoverNotifier {
    token: String,
    recipient: String,
}

impl fmt::Debug for PushoverNotifier {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("PushoverNotifier")
            .field("token", &"...")
            .field("recipient", &self.recipient)
            .finish()
    }
}

pub trait Notify {
    fn notify(&self, msg: &str) -> Result<(), Error>;
    fn notify_detailed(&self, title: &str, details: &str) -> Result<(), Error>;
}

impl PushoverNotifier {
    pub fn new(token: String, recipient: String) -> PushoverNotifier {
        PushoverNotifier { token, recipient }
    }
}

impl Notify for PushoverNotifier {
    fn notify(&self, msg: &str) -> Result<(), Error> {
        let client = Pushover::new(self.token.clone());
        client
            .request(&self.recipient, msg)
            .send()
            // We probably care about the return code or something, but we can deal with that later
            .map(|_| ())
    }
}

impl Notify for Option<PushoverNotifier> {
    fn notify(&self, msg: &str) -> Result<(), Error> {
        info!("sending push notification: {}", msg);
        match self {
            Some(notifier) => notifier.notify(msg),
            None => Ok(()),
        }
    }

    fn notify_detailed(&self, title: &str, details: &str) -> Result<(), Error> {
        info!("sending detailed push notification: {}", title);
        match self {
            Some(notifier) => notifier.notify_detailed(title, details),
            None => Ok(()),
        }
    }
}
