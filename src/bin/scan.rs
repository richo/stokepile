use clap::App;

use stokepile::cli;
use stokepile::config::Config;
use stokepile::ctx::Ctx;
use stokepile::ptp_device;

fn cli_opts<'a, 'b>() -> App<'a, 'b> {
    cli::base_opts()
        .about("Scans for attached devices and prints information found.")
}

fn main() {
    stokepile::cli::run(|| {

        let matches = cli_opts().get_matches();

        let cfg = Config::from_file(matches.value_of("config").unwrap_or("stokepile.toml"));
        let ctx = Ctx::create_without_lock(cfg?)?;
        println!("Found the following gopros:");

        for gopro in ptp_device::locate_gopros(&ctx)?.iter() {
            println!("  {:?} : {}", gopro.kind, gopro.serial);
        }

        Ok(())
    })
}
