use rocket::{get, post};
use rocket_contrib::templates::Template;
use rocket::response::{Flash, Redirect};
use rocket::request::Form;

use crate::web::auth::WebUser;
use crate::web::context::Context;
use crate::web::db::DbConn;

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
