#[macro_use]
extern crate log;

extern crate clap;
extern crate pretty_env_logger;
extern crate failure;

extern crate archiver;

use clap::{App, Arg, SubCommand};
use failure::Error;
use std::env;
use std::process;
use std::thread;

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
            println!("Attached devices:");
            for device in &devices {
                println!("  {:?}", device);
            }
            println!("");

            // Let the cameras populate the plan
            for device in devices {
                let msg = format!("Finished staging: {}", device.name());
                device.stage_files(&ctx.staging)?;
                ctx.notifier.notify(&msg)?;
            }

            // Run the uploader thread syncronously as a smoketest for the daemon mode
            let staging = ctx.staging.clone();
            let backend = ctx.cfg.backend().clone();
            let report = thread::spawn(move || storage::upload_from_staged(&staging, &backend))
                .join()
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
