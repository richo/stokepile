use failure::Error;

use pshovr;

#[derive(RedactedDebug)]
pub struct PushoverNotifier {
    recipient: String,
    client: pshovr::PushoverClient,
}

pub trait Notify {
    fn notify(&self, msg: &str) -> Result<(), Error>;
}

impl PushoverNotifier {
    pub fn new(token: String, recipient: String) -> PushoverNotifier {
        PushoverNotifier {
           recipient,
           client: pshovr::PushoverClient::new(token),
        }
    }
}

impl Notify for PushoverNotifier {
    fn notify(&self, msg: &str) -> Result<(), Error> {
        let notification = self.client.build_notification(&self.recipient, msg);
        self.client
            .send(&notification)
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
