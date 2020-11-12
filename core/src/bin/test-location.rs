use clap::{App, Arg};

use stokepile::config;
use stokepile::ctx::Ctx;
use stokepile::device;
use stokepile::cli;

fn cli_opts<'a, 'b>() -> App<'a, 'b> {
    cli::base_opts()
        .about("Smoke test the location and mounting")
}


fn main() {
    stokepile::cli::run(|| {
        let matches = cli_opts().get_matches();

        let cfg = config::Config::from_file(matches.value_of("config").unwrap_or("stokepile.toml"));
        let ctx = Ctx::create_without_lock(cfg?)?;

        let devices = device::attached_devices(&ctx)?;

        for device in devices {
            println!("  {:?}", &device);
        }

        Ok(())
    });
}
