use stokepile::web::db::run_migrations;

fn main() {
    stokepile::cli::run(|base| base, |_matches| {
        run_migrations()?;
        Ok(())
    })
}
