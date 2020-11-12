use stokepile::cli::init_dotenv;
use stokepile::web::db::run_migrations;

fn main() {
    stokepile::cli::run(|| {
        init_dotenv()?;
        run_migrations()?;
        Ok(())
    })
}
