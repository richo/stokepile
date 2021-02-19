use crate::web::db::DbConn;
use crate::web::auth::WebUser;
use crate::web::context::Context;

use rocket::request::FlashMessage;
use rocket_contrib::templates::Template;

#[get("/")]
pub fn index(user: Option<WebUser>, conn: DbConn, flash: Option<FlashMessage<'_, '_>>) -> Template {
    let context = Context::other()
        .set_user(user)
        .flash(flash.map(|ref msg| (msg.name().into(), msg.msg().into())));
    Template::render("index", context)
}

#[get("/privacy")]
pub fn privacy() -> Template {
    let context = Context::other();
    Template::render("privacy", context)
}

#[catch(404)]
pub fn not_found() -> Template {
    let context = Context::other();
    Template::render("404", context)
}
