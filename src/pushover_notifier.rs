use failure::Error;

use pushover::Pushover;

pub struct PushoverNotifier {
    token: String,
    recipient: String,
}

pub trait Notify {
    fn notify(&self, msg: &str) -> Result<(), Error>;
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
}
