#[macro_use]
extern crate log;

use clap::{App, Arg};

use stokepile::cli;
use stokepile::config;
use stokepile::ctx::Ctx;
use stokepile::device;
use stokepile::mailer::MailReport;
use stokepile::mountable::Mountable;
use stokepile::storage;

fn cli_opts<'a, 'b>() -> App<'a, 'b> {
    cli::base_opts()
        .about("Performs a single run, uploading footage from all connected devices")
        .arg(
            Arg::with_name("no-cron")
            .long("no-cron")
            .help("Don't invoke any of the locking machinery to ensure only one stokepile runs at a time")
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

        let staging_location = ctx.staging().mount()?;
        info!("Staging to {:?}", &staging_location);

        let stager = match cfg.preserve_device_files {
            true => Stager::preserving(staging_location),
            false => Stager::destructive(staging_location),
        };

        for device in devices {
            let msg = format!("Finished staging: {}", device.name());
            let num_files = device.stage_files(&stager)?;
            if num_files > 0 {
                if let Err(e) = ctx.notify(&msg) {
                    error!("Failed to send push notification: {:?}", e);
                }
            }
        }

        let report = storage::upload_from_staged(&staging, &backends)?;

        if report.num_uploads() > 0 {
            if let Err(e) = ctx.notify("Finished uploading media") {
                error!("Failed to send push notification: {:?}", e);
            }
        }

        let plaintext = report.to_plaintext()?;
        println!("{}", plaintext);

        if report.num_uploads() > 0 {
            if let Err(e) = ctx.mailer.send_report(&plaintext) {
                error!("Failed to send upload report: {:?}", e);
            }
        } else {
            info!("Not mailing report as no work was scheduled");
        }

        Ok(())
    })
}
