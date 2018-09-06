use serde::ser::{Serialize, Serializer};
use reqwest;
use failure::Error;

static MESSAGE_API_URL: &'static str = "https://api.pushover.net/1/messages.json";

#[derive(Debug)]
pub enum Priority {
    /// generate no notification/alert
    NoNotification,
    /// always send as a quiet notification
    QuietNotification,
    /// display as high-priority and bypass the user's quiet hours
    HighPriority,
    /// to also require confirmation from the user
    RequireConfirmation,
}

impl Serialize for Priority {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let pri = match self {
            Priority::NoNotification => -2,
            Priority::QuietNotification => -1,
            Priority::HighPriority => 1,
            Priority::RequireConfirmation => 2,
        };
        serializer.serialize_i8(pri)
    }
}

#[derive(Serialize,Debug)]
pub struct PushoverRequest<'a> {
    token: &'a str,
    user: &'a str,
    message: &'a str,
    // attachment - an image attachment to send with the message; see attachments for more information on how to upload files
    // device - your user's device name to send the message directly to that device, rather than all of the user's devices (multiple devices may be separated by a comma)
    title: Option<String>,
    url: Option<String>,
    url_title: Option<String>,
    priority: Option<Priority>,
    // sound - the name of one of the sounds supported by device clients to override the user's default sound choice
    // timestamp - a Unix timestamp of your message's date and time to display to the user, rather than the time your message is received by our API
}

macro_rules! setter {
    ($field:ident, $ty:ty) => {
        pub fn $field(mut self, $field: $ty) -> PushoverRequest<'a> {
            self.$field = Some($field);
            self
        }
    }
}

impl<'a> PushoverRequest<'a> {
    pub fn send(self) -> Result<reqwest::Response, Error> {
        let client = reqwest::Client::new();
        client.post(MESSAGE_API_URL)
            .form(&self)
            .send()
            .map_err(|e| format_err!("HTTP error: {:?}", e))
    }

    /// your message's title, otherwise your app's name is used
    setter!(title, String);
    /// a supplementary URL to show with your message
    setter!(url, String);
    /// a title for your supplementary URL, otherwise just the URL is shown
    setter!(url_title, String);
    /// The notification priority for this message
    setter!(priority, Priority);
}

pub struct Pushover {
    token: String,
}

impl Pushover {
    pub fn new(token: String) -> Pushover {
        Pushover {
            token,
        }
    }

    pub fn request<'a>(&'a self, user: &'a str, message: &'a str) -> PushoverRequest<'a> {
        PushoverRequest {
            token: &self.token,
            user,
            message,
            title: None,
            url: None,
            url_title: None,
            priority: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    use serde_json;

    #[test]
    fn test_serialized_priorities_dtrt() {
        let client = Pushover::new("".into());
        let req = client.request("".into(), "".into())
            .priority(Priority::HighPriority);
        assert!(serde_json::to_string(&req).unwrap().contains("\"priority\":1"),
                "Serialization failed");
    }

    #[test]
    #[ignore]
    fn test_sends_notification() {
        fn inner() -> Result<(), Error> {
            let pushover = Pushover::new(env::var("ARCHIVER_TEST_PUSHOVER_KEY").expect("Didn't provide test key"));
            let user_key: String = "redacted".into();
            pushover.request(&user_key, "hi there").send()?;
            Ok(())
        }

        if let Err(e) = inner() {
            panic!("{:?}", e);
        }
    }
}
