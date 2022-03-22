use stokepile::web::db::run_migrations;
use stokepile::cli::init_dotenv;
use stokepile::web::media_server::configure_rocket;

fn main() {
    stokepile::cli::run(|| {
        init_dotenv()?;
        configure_rocket().launch();
        Ok(())
    })
}
