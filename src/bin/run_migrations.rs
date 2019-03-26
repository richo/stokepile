use archiver::cli::init_dotenv;
use archiver::web::db::run_migrations;

fn main() {
    archiver::cli::run(|| {
        init_dotenv()?;
        run_migrations()?;
        Ok(())
    })
}
