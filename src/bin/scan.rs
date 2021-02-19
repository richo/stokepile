use clap::App;

use stokepile::config::Config;
use stokepile::ctx::Ctx;
use stokepile::ptp_device;

fn cli_opts<'a, 'b>(base: App<'a, 'b>) -> App<'a, 'b> {
    base.about("Scans for attached devices and prints information found.")
}

fn main() {
    stokepile::cli::run(cli_opts, |matches| {
        let cfg = Config::from_file(matches.value_of("config").unwrap_or("stokepile.toml"));
        let ctx = Ctx::create_without_lock(cfg?)?;
        println!("Found the following gopros:");

        for gopro in ptp_device::locate_gopros(&ctx)?.iter() {
            println!("  {:?} : {}", gopro.kind, gopro.serial);
        }

        Ok(())
    })
}
