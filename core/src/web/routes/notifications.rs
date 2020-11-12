use std::env;

use rocket_contrib::json::Json;

use crate::messages::{SendNotification, SendNotificationResp};
use crate::web::auth::ApiUser;
use crate::pushover_notifier::{Notify, PushoverNotifier};

lazy_static! {
    static ref PUSHOVER_TOKEN: String = {
        info!("Fetching pushover token from environment");
        env::var("STOKEPILE_PUSHOVER_TOKEN")
                .expect("Missing the STOKEPILE_PUSHOVER_TOKEN environment variable.")
    };
}

#[post("/notification/send", format = "json", data = "<notification>")]
pub fn notification_send(
    user: ApiUser,
    notification: Json<SendNotification>,
) -> Json<SendNotificationResp> {
    if let Some(recipient) = user.user.notify_pushover {
        let client = PushoverNotifier::new(PUSHOVER_TOKEN.clone(), recipient.clone());
        return match client.notify(&notification.message) {
            Ok(_) => Json(SendNotificationResp::Sent),
            Err(e) => Json(SendNotificationResp::Error(e.to_string())),
        };
    }
    Json(SendNotificationResp::NotConfigured)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web::test_helpers::*;

    use rocket::http::{ContentType, Header, Status};

    client_for_routes!(notification_send => client);

    #[test]
    fn test_doesnt_try_to_notify_when_not_configured() {
        let client = client();
        create_user(&client, "test1@email.com", "p@55w0rd");

        let token = signin_api(&client, "test1@email.com", "p@55w0rd").unwrap();

        let req = client
            .post("/notification/send")
            .header(ContentType::JSON)
            .header(Header::new("Authorization", format!("Bearer: {}", token)))
            .body("{\"message\": \"thisis a test message\"}");

        let mut response = req.dispatch();

        assert_eq!(response.status(), Status::Ok);
        let resp: SendNotificationResp =
            serde_json::from_str(&response.body_string().expect("Didn't recieve a body")).unwrap();
        assert_eq!(resp, SendNotificationResp::NotConfigured);
    }
}
