pub mod index;

use rocket::request::FlashMessage;
use rocket_contrib::templates::Template;

#[derive(Serialize)]
struct MediaContext {}

#[get("/")]
pub fn index(flash: Option<FlashMessage<'_, '_>>) -> Template {
    let ctx = MediaContext {};
    Template::render("media_server/index", ctx)
}
