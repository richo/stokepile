use archiver::web::db::run_migrations;

fn main() {
    archiver::cli::run(|| {
        run_migrations()?;
        Ok(())
    })
}
