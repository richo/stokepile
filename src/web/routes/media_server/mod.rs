pub mod api;

use rocket;
use rocket::request::FlashMessage;
use rocket_dyn_templates::Template;

#[derive(Serialize)]
struct MediaContext {}

#[rocket::get("/")]
pub fn index(flash: Option<FlashMessage<'_>>) -> Template {
    let ctx = MediaContext {};
    Template::render("media_server/index", ctx)
}
