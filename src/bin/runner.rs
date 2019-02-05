#[macro_use]
extern crate log;

use clap::{App, Arg};
use lockfile;
use std::thread;

use archiver::cli;
use archiver::config;
use archiver::ctx::Ctx;
use archiver::device;
use archiver::mailer::MailReport;
use archiver::pushover_notifier::Notify;
use archiver::storage;

fn cli_opts<'a, 'b>() -> App<'a, 'b> {
    cli::base_opts()
        .about("Performs a single run, uploading footage from all connected devices")
        .arg(
            Arg::with_name("no-cron")
            .long("no-cron")
            .help("Don't invoke any of the locking machinery to ensure only one archiver runs at a time")
            )
}

fn main() {
    archiver::cli::run(|| {
        let matches = cli_opts().get_matches();

        let cfg = config::Config::from_file(matches.value_of("config").unwrap_or("archiver.toml"));
        let is_cron = !matches.is_present("no-cron");

        let ctx = if is_cron {
            Ctx::create(cfg?)?
        } else {
            Ctx::create_without_lock(cfg?)?
        };

        let devices = device::attached_devices(&ctx)?;

        let work_to_be_done = devices.len() > 0;
        let maybe_notify = |msg: &str| {
            if work_to_be_done {
                if let Err(e) = ctx.notifier.notify(msg) {
                    error!("Failed to send push notification: {:?}", e);
                }
            }
        };

        // Send a notification if we're running in cron mode and we're starting an upload
        maybe_notify("Starting cron upload");

        info!("Attached devices:");
        for device in &devices {
            info!("  {:?}", device);
        }
        info!("");

        let backends = ctx.cfg.backends();
        info!("Configured backends:");
        for backend in &backends {
            info!("  {}", backend.name());
        }
        info!("");

        for device in devices {
            let msg = format!("Finished staging: {}", device.name());
            device.stage_files(&ctx.staging)?;
            maybe_notify(&msg);
        }

        // Run the uploader thread syncronously as a smoketest for the daemon mode
        let staging = ctx.staging.clone();
        let report = thread::spawn(move || storage::upload_from_staged(&staging, &backends))
            .join()
            .expect("Upload thread panicked")?;

        maybe_notify("Finished uploading media");

        let plaintext = report.to_plaintext()?;
        println!("{}", plaintext);

        if let Err(e) = ctx.mailer.send_report(&plaintext) {
            error!("Failed to send upload report: {:?}", e);
        }

        Ok(())
    })
}
