#[macro_use]
extern crate log;

use std::io::Read;

use archiver::config;
use archiver::ctx::Ctx;
use archiver::device;
use archiver::mailer::MailReport;
use archiver::mountable::Mountable;
use archiver::storage;

fn main() {
    archiver::cli::run_and_wait(|| {
        let cfg = config::Config::from_file("archiver.toml");
        let ctx = Ctx::create_without_lock(cfg?)?;

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

        let staging = ctx.staging().mount()?;
        info!("Staging to {:?}", &staging);

        for device in devices {
            let msg = format!("Finished staging: {}", device.name());
            let num_files = device.stage_files(&staging)?;
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
