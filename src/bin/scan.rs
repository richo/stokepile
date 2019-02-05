use clap::App;

use archiver::cli;
use archiver::config::Config;
use archiver::ctx::Ctx;
use archiver::ptp_device;

fn cli_opts<'a, 'b>() -> App<'a, 'b> {
    cli::base_opts()
        .about("Scans for attached devices and prints information found.")
}

fn main() {
    archiver::cli::run(|| {

        let matches = cli_opts().get_matches();

        let cfg = Config::from_file(matches.value_of("config").unwrap_or("archiver.toml"));
        let ctx = Ctx::create(cfg?)?;
        println!("Found the following gopros:");

        for gopro in ptp_device::locate_gopros(&ctx)?.iter() {
            println!("  {:?} : {}", gopro.kind, gopro.serial);
        }

        Ok(())
    })
}
