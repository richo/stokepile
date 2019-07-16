use stokepile::config;
use stokepile::ctx::Ctx;

fn main() {
    stokepile::cli::run(|| {
        let cfg = config::Config::from_file("stokepile.toml");
        let ctx = Ctx::create(cfg?)?;

        ctx.notify("Test notification!")?;

        Ok(())
    })
}
