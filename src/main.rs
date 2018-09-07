#![deny(unused_must_use)]

extern crate serde;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate failure;
#[macro_use] extern crate handlebars;
#[macro_use] extern crate hyper;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;

extern crate chrono;
extern crate clap;
extern crate digest;
extern crate hashing_copy;
extern crate hex;
extern crate libusb;
extern crate pretty_env_logger;
extern crate ptp;
extern crate regex;
extern crate reqwest;
extern crate serde_json;
extern crate sendgrid;
extern crate sha2;
extern crate toml;
extern crate walkdir;

mod dropbox_content_hasher;

use clap::{App,SubCommand,Arg};
use failure::Error;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process;
use std::thread;

mod config;
mod ctx;
mod device;
mod dropbox;
mod flysight;
mod mass_storage;
mod peripheral;
mod pushover;
mod pushover_notifier;
mod ptp_device;
mod reporting;
mod staging;
mod storage;
mod version;

use pushover_notifier::Notify;

fn cli_opts<'a, 'b>() -> App<'a, 'b> {
    App::new("archiver")
        .version(version::VERSION)
        .about("Footage archiver")
        .author("richö butts")
        .arg(Arg::with_name("config")
             .long("config")
             .takes_value(true)
             .help("Path to configuration file"))
        .subcommand(SubCommand::with_name("daemon")
                    .version(version::VERSION)
                    .author("richö butts")
                    .about("Runs archiver in persistent mode"))
        .subcommand(SubCommand::with_name("scan")
                    .version(version::VERSION)
                    .author("richö butts")
                    .about("Scan for attached devices"))
        .subcommand(SubCommand::with_name("run")
                    .about("Runs archiver in persistent mode")
                    .version(version::VERSION)
                    .author("richö butts")
                    .arg(Arg::with_name("plan-only")
                         .long("plan-only")
                         .help("Don't upload, only create a plan"))
                    )
}

fn create_ctx(matches: &clap::ArgMatches) -> Result<ctx::Ctx, Error> {
    let usb_ctx = libusb::Context::new()?;
    let cfg = config::Config::from_file(matches.value_of("config").unwrap_or("archiver.toml"))?;
    let notifier = cfg.pushover();
    let staging = create_or_find_staging(&cfg)?;
    Ok(ctx::Ctx {
        // Loading config here lets us bail at a convenient time before we get in the weeds
        usb_ctx,
        cfg,
        staging,
        notifier,
    })
}

fn init_logging() {
    if let None = env::var_os("RUST_LOG") {
        env::set_var("RUST_LOG", "INFO");
        pretty_env_logger::init();
    }
}

fn create_or_find_staging(cfg: &config::Config) -> Result<PathBuf, Error> {
    let path = cfg.staging_dir()?.unwrap_or_else(|| PathBuf::from("staging"));

    if let Err(e) = fs::create_dir(&path) {
        if e.kind() == io::ErrorKind::AlreadyExists {
            info!("Reusing existing staging dir");
        } else {
            error!("{:?}", e);
            panic!();
        }
    }

    Ok(path)
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
    let ctx = create_ctx(&matches)?;

    match matches.subcommand() {
        ("daemon", Some(_subm))  => {
            unimplemented!();
        },
        ("run", Some(subm)) => {
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
            let report = thread::spawn(move || storage::upload_from_staged(&staging, &backend)).join().expect("Upload thread panicked")?;
            ctx.notifier.notify("Finished uploading media")?;
            println!("{}", report.to_plaintext()?);
        },
        ("scan", Some(_subm)) => {
            println!("Found the following gopros:");
            for gopro in ptp_device::locate_gopros(&ctx)?.iter() {
                println!("  {:?} : {}", gopro.kind, gopro.serial);
            }
        },
        _ => {
            error!("No subcommand provided");
            unreachable!();
        }, // Either no subcommand or one not tested for...
    }

    Ok(())
}
