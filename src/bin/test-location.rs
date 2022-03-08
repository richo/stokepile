use clap::App;

use stokepile::config;
use stokepile::ctx::Ctx;
use stokepile::device;

fn cli_opts<'a, 'b>(base: App<'a, 'b>) -> App<'a, 'b> {
    base.about("Smoke test the location and mounting")
}


fn main() {
    stokepile::cli::run(cli_opts, |matches| {
        let cfg = config::Config::from_file(matches.value_of("config").unwrap_or("stokepile.toml"));
        let ctx = Ctx::create_without_lock(cfg?)?;

        let devices = device::attached_devices(&ctx)?;

        for device in devices {
            println!("  {:?}", &device);
        }

        Ok(())
    });
}
