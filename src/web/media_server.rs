use rocket::Rocket;
use rocket_contrib::serve::StaticFiles;

use crate::web::routes::media_server as routes;
use crate::ctx::Ctx;
use crate::config::Config;
use crate::mountable::Mountable;

use failure::Error;

fn get_ctx() -> Result<Ctx, Error> {
    let cfg = Config::from_file("stokepile.toml");
    // TODO(richo)
    Ok(Ctx::create_without_lock(cfg?)?)
}

pub fn configure_rocket() -> Rocket {
    let ctx = get_ctx().expect("Couldn't get ctx");
    let staging = ctx.staging().mount().expect("Couldn't mount staging");
    super::configure_rocket(
        routes![
            routes::index,
            routes::api::get_media,
        ]
    )
    .manage(ctx)
    .manage(staging)
    .mount("/wasm", StaticFiles::from("wasm/pkg"))
}
