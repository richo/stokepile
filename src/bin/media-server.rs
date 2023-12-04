use rocket::launch;
use stokepile::cli::init_dotenv;
use stokepile::web::media_server::configure_rocket;

#[launch]
fn entry() -> _ {
    stokepile::cli::run(|| {
        init_dotenv()?;
        Ok(configure_rocket())
    })
}
