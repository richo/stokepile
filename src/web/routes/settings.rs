use std::path::PathBuf;

use rocket::{get, post};
use rocket_contrib::templates::Template;
use rocket::response::{Flash, Redirect};
use rocket::request::Form;

use crate::config::{MountableDeviceLocation, StagingConfig};
use crate::web::auth::WebUser;
use crate::web::context::Context;
use crate::web::db::DbConn;
use crate::web::models::extra::StagingKind;

#[get("/settings")]
pub fn get_settings(user: WebUser) -> Template {
    let context = Context::media(())
        .set_user(Some(user));
    Template::render("settings", context)
}

#[post("/settings",  data = "<settings>")]
pub fn post_settings(user: WebUser, conn: DbConn, settings: Form<SettingsForm>) -> Flash<Redirect> {

    match user.user.update_from_settings(&*settings, &conn) {
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

#[derive(FromForm, Debug, Serialize)]
pub struct SettingsForm {
    pub(crate) notification_email: String,
    pub(crate) notification_pushover: String,
    pub(crate) staging_data: String,
    pub(crate) staging_type: StagingKind,
    pub(crate) preserve_device_files: bool,
}

impl SettingsForm {
    /// Coerce the separate values given in the form back into a StagingConfig
    pub fn staging(&self) -> Option<StagingConfig> {
        if self.staging_data.len() == 0 {
            return None;
        }
        let location = match self.staging_type {
            StagingKind::None => return None,
            StagingKind::Label => MountableDeviceLocation::Label(self.staging_data.clone()),
            StagingKind::Mountpoint => {
                let pathbuf = PathBuf::from(&self.staging_data);
                MountableDeviceLocation::Mountpoint(pathbuf)
            },
            StagingKind::Location => {
                let pathbuf = PathBuf::from(&self.staging_data);
                MountableDeviceLocation::Location(pathbuf)
            },
        };
        Some(StagingConfig {
            location,
        })
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
    use serde_urlencoded;

    client_for_routes!(config: get_settings, post_settings => client);

    #[test]
    fn test_can_set_and_unset_settings() {
        use crate::web::schema::users::dsl::{users, id};

        init_env();
        let client = client();
        let user = create_user(&client, "test1@email.com", "p@55w0rd");
        let _session = signin(&client, "test1%40email.com", "p%4055w0rd").unwrap();

        assert_eq!(None, user.notify_email);
        assert_eq!(None, user.notify_pushover);

        let settings = SettingsForm {
            notification_email: "test-value".into(),
            notification_pushover: "another test value".into(),
            staging_type: StagingKind::Label,
            staging_data: "BUTTS".into(),
            preserve_device_files: false,
        };
        let serialized = serde_urlencoded::to_string(&settings).expect("Couldn't serialize form");

        let response = client
            .post("/settings")
            .header(ContentType::Form)
            .body(serialized.as_bytes())
            .dispatch();
        assert_eq!(response.status(), Status::SeeOther);

        // Reload the user. There is probably a better way to do this.
        let user = {
            let conn = db_conn(&client);

            users.filter(id.eq(user.id)).get_result::<User>(&*conn).unwrap()
        };

        assert_eq!(Some("test-value".into()), user.notify_email);
        assert_eq!(Some("another test value".into()), user.notify_pushover);

        assert_eq!(Some("BUTTS".into()), user.staging_data);
        assert_eq!(StagingKind::Label, user.staging_type);

        let settings = SettingsForm {
            notification_email: "".into(),
            notification_pushover: "".into(),
            staging_type: StagingKind::None,
            staging_data: "".into(),
            preserve_device_files: false,
        };
        let serialized = serde_urlencoded::to_string(&settings).expect("Couldn't serialize form");

        let response = client
            .post("/settings")
            .header(ContentType::Form)
            .body(serialized.as_bytes())
            .dispatch();
        assert_eq!(response.status(), Status::SeeOther);

        // Reload the user. There is probably a better way to do this.
        let user = {
            let conn = db_conn(&client);

            users.filter(id.eq(user.id)).get_result::<User>(&*conn).unwrap()
        };

        assert_eq!(None, user.notify_email);
        assert_eq!(None, user.notify_pushover);

        assert_eq!(None, user.staging_data);
        assert_eq!(StagingKind::None, user.staging_type);
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

        let settings = SettingsForm {
            notification_email: "lol".into(),
            notification_pushover: "hithere".into(),
            staging_type: StagingKind::Mountpoint,
            staging_data: "butts".into(),
            preserve_device_files: false,
        };
        let serialized = serde_urlencoded::to_string(&settings).expect("Couldn't serialize form");

        let response = client1
            .post("/settings")
            .header(ContentType::Form)
            .body(serialized)
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
