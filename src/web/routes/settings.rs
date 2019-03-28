use std::path::PathBuf;

use rocket::{get, post};
use rocket_contrib::templates::Template;
use rocket::response::{Flash, Redirect};
use rocket::request::Form;

use crate::config::StagingConfig;
use crate::web::auth::WebUser;
use crate::web::context::Context;
use crate::web::db::DbConn;
use crate::web::models::extra::StagingKind;

#[get("/settings")]
pub fn get_settings(user: WebUser) -> Template {
    let context = Context::default()
        .set_user(Some(user));
    Template::render("settings", context)
}

#[post("/settings",  data = "<settings>")]
pub fn post_settings(user: WebUser, conn: DbConn, settings: Form<SettingsForm>) -> Flash<Redirect> {

    match user.user.update_settings(&settings, &conn) {
        Ok(_) => {
            Flash::success(
                Redirect::to("/"),
                "Settings updated.",
                )
        },
        Err(e) => {
            Flash::error(
                Redirect::to("/settings"),
                format!("Error updating settings, {:?}", e),
                )
        }
    }
}

#[derive(FromForm, Debug)]
pub struct SettingsForm {
    notification_email: String,
    notification_pushover: String,
    staging_location: String,
    staging_type: StagingKind,
}

impl SettingsForm {
    /// Coerce the separate values given in the form back into a StagingConfig
    pub fn staging(&self) -> StagingConfig {
        let location = match self.staging_type {
            StagingKind::Device => MountableDeviceLocation::Label(self.staging_location.clone()),
            StagingKind::Directory => {
                let pathbuf = PathBuf::from(&self.staging_location);
                MountableDeviceLocation::Mountpoint(pathbuf)
            }
        };
        StagingConfig {
            location,
        }
    }
}

impl SettingsForm {
    pub fn notification_email(&self) -> Option<&str> {
        if self.notification_email.len() > 0 {
            Some(&self.notification_email)
        } else {
            None
        }
    }

    pub fn notification_pushover(&self) -> Option<&str> {
        if self.notification_pushover.len() > 0 {
            Some(&self.notification_pushover)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web::test_helpers::*;
    use crate::web::models::User;
    use diesel::prelude::*;

    use rocket::http::{ContentType, Status};

    client_for_routes!(get_settings, post_settings => client);

    #[test]
    fn test_can_set_and_unset_settings() {
        use crate::web::schema::users::dsl::{users, id};

        init_env();
        let client = client();
        let user = create_user(&client, "test1@email.com", "p@55w0rd");
        let _session = signin(&client, "test1%40email.com", "p%4055w0rd").unwrap();

        assert_eq!(None, user.notify_email);
        assert_eq!(None, user.notify_pushover);

        // Set some settings
        let response = client
            .post("/settings")
            .header(ContentType::Form)
            .body(r"notification_email=test-value&notification_pushover=another%20test%20value&staging_type=device&staging_location=/butts")
            .dispatch();
        assert_eq!(response.status(), Status::SeeOther);

        // Reload the user. There is probably a better way to do this.
        let user = {
            let conn = db_conn(&client);

            users.filter(id.eq(user.id)).get_result::<User>(&*conn).unwrap()
        };

        assert_eq!(Some("test-value".into()), user.notify_email);
        assert_eq!(Some("another test value".into()), user.notify_pushover);
    }


    #[test]
    fn test_connect_integration_doesnt_stomp_on_sessions() {
        use crate::web::schema::users::dsl::{users, id};
        init_env();

        let client1 = client();
        let client2 = client();
        let u1 = create_user(&client1, "test1@email.com", "p@55w0rd");
        let u2 = create_user(&client2, "test2@email.com", "p@55w0rd");

        let _s1 = signin(&client1, "test1%40email.com", "p%4055w0rd").unwrap();
        let _s2 = signin(&client2, "test2%40email.com", "p%4055w0rd").unwrap();

        // Set some settings
        let response = client1
            .post("/settings")
            .header(ContentType::Form)
            .body(r"notification_email=lol&notification_pushover=hithere&staging_type=device&staging_location=/butts")
            .dispatch();
        assert_eq!(response.status(), Status::SeeOther);

        let u1 = {
            let conn = db_conn(&client1);
            users.filter(id.eq(u1.id)).get_result::<User>(&*conn).unwrap()
        };
        let u2 = {
            let conn = db_conn(&client2);
            users.filter(id.eq(u2.id)).get_result::<User>(&*conn).unwrap()
        };

        assert_eq!(None, u2.notify_email);
        assert_eq!(None, u2.notify_pushover);

        assert_eq!(Some("lol".into()), u1.notify_email);
        assert_eq!(Some("hithere".into()), u1.notify_pushover);
    }
}
