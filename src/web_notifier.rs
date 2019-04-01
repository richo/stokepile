use failure::Error;

use crate::config;
use crate::client::ArchiverClient;
use crate::pushover_notifier::Notify;

#[derive(Debug)]
pub struct WebNotifier {
    client: ArchiverClient,
}

impl WebNotifier {
    pub fn new(cfg: &config::Config) -> Result<WebNotifier, Error> {
        let base = cfg.api_base();
        let mut client = ArchiverClient::new(&base)?;
        client.load_token()?;

        Ok(WebNotifier {
            client,
        })
    }
}

impl Notify for WebNotifier {
    fn notify(&self, msg: &str) -> Result<(), Error> {
        self.client.send_notification(msg)
    }
}

