use rocket::{Request, Data};
use rocket::fairing::{Fairing, Info, Kind};

// Purely for the log struct
use serde_json;
use std::net::IpAddr;

use async_trait::async_trait;


#[derive(Debug)]
pub struct RequestLogger {
}

impl RequestLogger {
    pub fn new() -> Self {
        RequestLogger {}
    }
}

#[derive(Serialize, Debug)]
struct LoggedRequest<'a> {
    client_ip: Option<IpAddr>,
    real_ip: Option<IpAddr>,
    method: &'a str,
    uri: String,
}

impl<'a> LoggedRequest<'a> {
    fn from_request(req: &Request<'_>) -> Self {
        LoggedRequest {
            client_ip: req.client_ip(),
            real_ip: req.real_ip(),
            method: req.method().as_str(),
            uri: req.uri().to_string(),
        }
    }
}

#[async_trait]
impl Fairing for RequestLogger {
    fn info(&self) -> Info {
        Info {
            name: "Request logger",
            kind: Kind::Request,
        }
    }

    // Spit out some structured logs about who's making this request
    async fn on_request(&self, request: &mut Request<'_>, _: &mut Data<'_>) {
        // We don't want to log requests for statics.
        if request.uri().path().starts_with("/static/") {
            return
        }

        // Let's eventually figure out what user we are as well.
        let log = LoggedRequest::from_request(request);

        match serde_json::to_string(&log) {
            Ok(log) => info!("{}", log),
            Err(e) => warn!("Failed to serialize request log: {:?}", e),
        };
    }
}
