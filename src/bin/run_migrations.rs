use stokepile::web::db::run_migrations;

fn main() {
    stokepile::cli::run(|| {
        run_migrations()?;
        Ok(())
    })
}
