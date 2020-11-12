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

use std::io;

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

        let staging_location = ctx.staging().mount()?;
        info!("Staging to {:?}", &staging_location);

        let stager = match ctx.cfg.preserve_device_files() {
            true => Stager::preserving(staging_location),
            false => Stager::destructive(staging_location),
        };

        for device in devices {
            let finish_msg = format!("Finished staging: {}", device.name());
            let incomplete_msg = format!("Partially staged {}, device will need a second run", device.name());
            let notify = |msg: String| {
                if let Err(e) = ctx.notify(&msg) {
                    error!("Failed to send push notification: {:?}", e);
                }
            };
            match device.stage_files(&stager) {
                Ok(num_files) => {
                    if num_files > 0 {
                        notify(finish_msg);
                    }
                },
                // TODO(richo) We probably want to just have this be an io::Error instead of
                // faffing about with failure. As it stands, we use .context() to help with
                // debugging which more or less implies failure, but maybe there's some
                // middleground.
                Err(err) => {
                    if let Some(err) = err.downcast_ref::<io::Error>() {
                        if err.kind() == io::ErrorKind::Interrupted {
                            warn!("Staging device full, continuing");
                            notify(incomplete_msg);
                            break
                        }
                    }
                    Err(err)?
                }
            }
        }

        if matches.is_present("stage-only") {
            info!("Not uploading any data");
            return Ok(());
        }

        let report = storage::upload_from_staged(&stager.staging_location(), &backends)?;

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
