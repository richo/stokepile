use archiver::config;
use archiver::ctx::Ctx;
use archiver::pushover_notifier::Notify;

fn main() {
    archiver::cli::run(|| {
        let cfg = config::Config::from_file("archiver.toml");
        let ctx = Ctx::create(cfg?)?;

        ctx.notifier.notify("Test notification!")?;

        Ok(())
    })
}
