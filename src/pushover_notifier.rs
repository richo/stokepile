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
        info!("Sending push notification: {}", msg);
        let notification = self.client.build_notification(&self.recipient, msg);
        self.client
            .send(&notification)
            .map(|_| ())
    }
}

impl<T> Notify for Option<T> where T: Notify {
    fn notify(&self, msg: &str) -> Result<(), Error> {
        match self {
            Some(notifier) => notifier.notify(msg),
            None => {
                info!("Notifier not configured, ignoring push notification: {}", msg);
                Ok(())
            }
        }
    }
}
