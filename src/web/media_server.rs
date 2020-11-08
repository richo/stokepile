use rocket::Rocket;
use rocket_contrib::serve::StaticFiles;

use crate::web::routes::media_server as routes;

pub fn configure_rocket() -> Rocket {
    super::configure_rocket(
        routes![
            routes::index,
        ]
    )
    .mount("/wasm", StaticFiles::from("wasm/pkg"))
}
