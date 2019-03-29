use std::env;

use rocket_contrib::json::Json;

use crate::messages::{SendNotification, SendNotificationResp};
use crate::web::db::DbConn;
use crate::web::auth::ApiUser;
use crate::pushover_notifier::{Notify, PushoverNotifier};

lazy_static! {
    static ref PUSHOVER_TOKEN: String = {
        info!("Fetching pushover token from environment");
        env::var("ARCHIVER_PUSHOVER_TOKEN")
                .expect("Missing the ARCHIVER_PUSHOVER_TOKEN environment variable.")
    };
}

#[post("/notification/send", format = "json", data = "<notification>")]
pub fn notification_send(
    conn: DbConn,
    user: ApiUser,
    notification: Json<SendNotification>,
) -> Json<SendNotificationResp> {
    warn!("input: user: {:?} notification: {:?}", &user, &notification);
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

    use rocket::http::{ContentType, Status};

    client_for_routes!(notification_send => client);

    #[test]
    fn test_doesnt_try_to_notify_when_not_configured() {
        init_env();
        let client = client();
        let user = create_user(&client, "test1@email.com", "p@55w0rd");

        // {
        //     let conn = db_conn(&client1);
        //     users.update(id.eq(user.id)).set((users.notify_pushover("1234"))
        // }

        signin(&client, "test1%40email.com", "p%4055w0rd").unwrap();

        let req = client
            .post("/notification/send")
            .header(ContentType::JSON)
            .body("{\"message\": \"thisis a test message\"}");

        let mut response = req.dispatch();

        assert_eq!(response.status(), Status::Ok);
        let resp: SendNotificationResp =
            serde_json::from_str(&response.body_string().expect("Didn't recieve a body")).unwrap();
        assert_eq!(resp, SendNotificationResp::NotConfigured);
    }
}
