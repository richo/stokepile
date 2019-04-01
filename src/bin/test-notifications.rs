use archiver::config;
use archiver::ctx::Ctx;

fn main() {
    archiver::cli::run(|| {
        let cfg = config::Config::from_file("archiver.toml");
        let ctx = Ctx::create(cfg?)?;

        ctx.notify("Test notification!")?;

        Ok(())
    })
}
