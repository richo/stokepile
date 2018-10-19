#[macro_use]
extern crate log;
#[macro_use]
extern crate failure;

extern crate clap;
extern crate pretty_env_logger;

extern crate archiver;

use clap::{App, Arg, SubCommand};
use failure::Error;
use std::env;
use std::process;
use std::thread;
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::time::Duration;

use archiver::config;
use archiver::ctx::Ctx;
use archiver::device;
use archiver::mailer::MailReport;
use archiver::pushover_notifier::Notify;
use archiver::ptp_device;
use archiver::storage;
use archiver::VERSION;

fn cli_opts<'a, 'b>() -> App<'a, 'b> {
    App::new("archiver")
        .version(VERSION)
        .about("Footage archiver")
        .author("richö butts")
        .arg(
            Arg::with_name("config")
                .long("config")
                .takes_value(true)
                .help("Path to configuration file"),
        ).subcommand(
            SubCommand::with_name("daemon")
                .version(VERSION)
                .author("richö butts")
                .about("Runs archiver in persistent mode"),
        ).subcommand(
            SubCommand::with_name("scan")
                .version(VERSION)
                .author("richö butts")
                .about("Scan for attached devices"),
        ).subcommand(
            SubCommand::with_name("run")
                .about("Runs archiver in persistent mode")
                .version(VERSION)
                .author("richö butts")
                .arg(
                    Arg::with_name("plan-only")
                        .long("plan-only")
                        .help("Don't upload, only create a plan"),
                ),
        )
}

fn init_logging() {
    if let None = env::var_os("RUST_LOG") {
        env::set_var("RUST_LOG", "INFO");
    }
    pretty_env_logger::init();
}

fn main() {
    init_logging();
    if let Err(e) = run() {
        error!("Error running archiver");
        error!("{:?}", e);
        if env::var("RUST_BACKTRACE").is_ok() {
            error!("{:?}", e.backtrace());
        }
        process::exit(1);
    }
}

fn run() -> Result<(), Error> {
    let matches = cli_opts().get_matches();

    // TODO(richo) run -> Result<(), Error> so I can use ?
    let cfg = config::Config::from_file(matches.value_of("config").unwrap_or("archiver.toml"))?;
    let ctx = Ctx::create(cfg)?;

    match matches.subcommand() {
        ("daemon", Some(_subm)) => {
            unimplemented!();
        }
        ("run", Some(_subm)) => {
            let devices = device::attached_devices(&ctx)?;
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

            // Create a channel to notify the upload thread when we have finished staging
            let (done, rx) = channel();

            // Create a channel to notify the upload thread when we have finished staging
            let (finished, reports) = channel::<Result<String, String>>();

            // Launch the uploader thread. We try uploading from staging, and then if we're not
            // done we slee
            info!("Starting upload thread");
            let staging = ctx.staging.clone();
            let report_thread = thread::spawn(move || {
                let mut done = false;
                loop {
                    if done {
                        return storage::upload_from_staged(&staging, &backends);
                    }
                    // See if we've finished yet, or sleep for 10 seconds
                    match rx.recv_timeout(Duration::new(10, 0)) {
                        Ok(_) => {
                            info!("Staging finished. Triggering a last upload run and exiting");
                            done = true;
                        },
                        Err(RecvTimeoutError::Disconnected) => {
                            info!("Parent thread crashed. Exiting to avoid inconsistent state");
                            return Err(format_err!("Parent thread crashed"));
                        },
                        Err(RecvTimeoutError::Timeout) => {
                            debug!("Didn't get a response from upstream in 10s");
                        },
                    }
                }

            });

            // Let the cameras populate the plan
            info!("Starting staging");
            for device in devices {
                let msg = format!("Finished staging: {}", device.name());
                device.stage_files(&ctx.staging)?;
                ctx.notifier.notify(&msg)?;
            }
            ctx.notifier.notify("Finished staging media")?;
            done.send(());

            let report = report_thread.join()
                .expect("Upload thread panicked")?;
            ctx.notifier.notify("Finished uploading media")?;
            let plaintext = report.to_plaintext()?;
            println!("{}", plaintext);
            ctx.mailer.send_report(&plaintext)?;
        }
        ("scan", Some(_subm)) => {
            println!("Found the following gopros:");
            for gopro in ptp_device::locate_gopros(&ctx)?.iter() {
                println!("  {:?} : {}", gopro.kind, gopro.serial);
            }
        }
        _ => {
            error!("No subcommand provided");
            unreachable!();
        } // Either no subcommand or one not tested for...
    }

    Ok(())
}
