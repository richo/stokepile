use std::path::PathBuf;

use rocket::{get, post};
use rocket_contrib::templates::Template;
use rocket::response::{Flash, Redirect};
use rocket::request::Form;

use crate::web::auth::WebUser;
use crate::web::context::Context;
use crate::web::db::DbConn;


#[get("/settings")]
pub fn get_settings(user: WebUser) -> Template {
    let context = Context::rigging(())
        .set_user(Some(user));
    Template::render("rigging/settings", context)
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
    pub(crate) certificate: String,
    pub(crate) seal: String,
}
