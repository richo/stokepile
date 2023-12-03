use failure::Error;

use crate::config;
use crate::client::StokepileClient;
use crate::pushover_notifier::Notify;
use crate::async_hacks;

#[derive(Debug)]
pub struct WebNotifier {
    client: StokepileClient,
}

impl WebNotifier {
    pub fn new(cfg: &config::Config) -> Result<WebNotifier, Error> {
        let base = cfg.api_base();
        let mut client = StokepileClient::new(&base)?;
        client.load_token()?;

        Ok(WebNotifier {
            client,
        })
    }
}

impl Notify for WebNotifier {
    fn notify(&self, msg: &str) -> Result<(), Error> {
        async_hacks::block_on(self.client.send_notification(msg))
    }
}

