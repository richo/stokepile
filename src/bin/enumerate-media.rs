#[macro_use]
extern crate log;

use clap::{App, Arg};

use stokepile::cli;
use stokepile::config;
use stokepile::ctx::Ctx;
use stokepile::device;
use stokepile::mailer::MailReport;
use stokepile::mountable::Mountable;
use stokepile::staging::Stager;
use stokepile::storage;

fn cli_opts<'a, 'b>() -> App<'a, 'b> {
    cli::base_opts()
        .about("Performs a single run, uploading footage from all connected devices")
        .arg(
            Arg::with_name("no-cron")
            .long("no-cron")
            .help("Don't invoke any of the locking machinery to ensure only one stokepile runs at a time")
            )
        .arg(
            Arg::with_name("stage-only")
            .long("stage-only")
            .help("Only stage files, do not process uploads")
            )
}

fn main() {
    stokepile::cli::run(|| {
        let matches = cli_opts().get_matches();

        let cfg = config::Config::from_file(matches.value_of("config").unwrap_or("stokepile.toml"));
        let is_cron = !matches.is_present("no-cron");

        let ctx = if is_cron {
            Ctx::create(cfg?)?
        } else {
            Ctx::create_without_lock(cfg?)?
        };

        let devices = device::attached_devices(&ctx)?;

        info!("Attached devices:");
        for device in &devices {
            info!("  {:?}", device);
        }
        info!("");

        let backends = ctx.cfg.backends();
        info!("Configured backends:");
        for backend in &backends {
            info!("  {:?}", backend);
        }
        info!("");

        for device in devices {
            info!("Device: {}", device.name());
            for file in device.mass_storage_files()? {
                info!("  {:?}", &file);
            }
        }

        Ok(())
    })
}
