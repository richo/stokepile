use rocket::Rocket;
use rocket_contrib::serve::StaticFiles;

use crate::web::routes::media_server as routes;
use crate::ctx::Ctx;
use crate::config::Config;
use crate::mountable::Mountable;

use std::thread::JoinHandle;
use std::sync::RwLock;

use failure::Error;

#[derive(Debug)]
pub enum OperationalState {
    Idle,
    Transforming(JoinHandle<Option<Error>>),
    Uploading(JoinHandle<Option<Error>>),
}

#[derive(Debug)]
pub struct OperationalStatus {
    state: RwLock<OperationalState>,
}

impl OperationalStatus {
    fn new() -> Self {
        OperationalStatus {
            state: RwLock::new(OperationalState::Idle)
        }
    }
}

fn get_ctx() -> Result<Ctx, Error> {
    let cfg = Config::from_file("stokepile.toml");
    // TODO(richo)
    Ok(Ctx::create_without_lock(cfg?)?)
}

pub fn configure_rocket() -> Rocket {
    let ctx = get_ctx().expect("Couldn't get ctx");
    let staging = ctx.staging().mount().expect("Couldn't mount staging");
    let status = OperationalStatus::new();

    super::configure_rocket(
        routes![
            routes::index,
            routes::api::get_media,
            routes::api::stream_media,
            routes::api::update_media,
            routes::api::apply_trims,
        ]
    )
    .manage(ctx)
    .manage(staging)
    .mount("/wasm", StaticFiles::from("wasm/pkg"))
    .mount("/vendor", StaticFiles::from("web/vendor"))
}
