use clap::{App, Arg};

use archiver::config;
use archiver::ctx::Ctx;
use archiver::device;
use archiver::cli;

fn cli_opts<'a, 'b>() -> App<'a, 'b> {
    cli::base_opts()
        .about("Smoke test the location and mounting")
}


fn main() {
    archiver::cli::run(|| {
        let matches = cli_opts().get_matches();

        let cfg = config::Config::from_file(matches.value_of("config").unwrap_or("archiver.toml"));
        let ctx = Ctx::create_without_lock(cfg?)?;

        let devices = device::attached_devices(&ctx)?;

        for device in devices {
            println!("  {:?}", &device);
        }

        Ok(())
    });
}
