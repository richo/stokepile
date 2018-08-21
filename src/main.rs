#[macro_use] extern crate serde_derive;
#[macro_use] extern crate failure;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate hyper;

extern crate serde_json;
extern crate regex;


use std::process;

extern crate clap;
use clap::{App,SubCommand,Arg};

extern crate libusb;
extern crate chrono;

extern crate reqwest;

use failure::Error;

mod config;
mod ctx;
mod device;
mod dropbox;
mod peripheral;
mod plan;
mod ptp_device;
mod version;

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
                    // .arg(Arg::with_name("device")
                    //      .short("d")
                    //      .takes_value(true)
                    //      .multiple(true)
                    //      .help("Upload only from device"))
                    )
}

fn create_ctx(matches: &clap::ArgMatches) -> Result<ctx::Ctx, Error> {
    Ok(ctx::Ctx {
        // Loading config here lets us bail at a convenient time before we get in the weeds
        usb_ctx: libusb::Context::new()?,
        cfg: config::Config::from_file(matches.value_of("config").unwrap_or("archiver.toml"))?,
    })
}

fn main() {
    if let Err(e) = run() {
        println!("Error running archiver");
        println!("{}", e);
        process::exit(1);
    }
}

fn run() -> Result<(), Error> {
    let matches = cli_opts().get_matches();

    // TODO(richo) run -> Result<(), Error> so I can use ?
    let ctx = create_ctx(&matches)?;

    match matches.subcommand() {
        ("daemon", Some(subm))  => {
            unimplemented!();
        },
        ("run", Some(subm)) => {
            let mut plan = plan::UploadPlan::new();
            // Figure out which cameras we're gunna be operating on
            let devices = device::attached_devices(&ctx)?;
            println!("Attached devices:");
            println!("{:?}", devices);

            // Let the cameras populate the plan
            for device in devices {
                plan.update(device);
            }

            plan.execute();
        },

        ("scan", Some(subm)) => {
            println!("Found the following gopros:");
            for gopro in ptp_device::locate_gopros(&ctx)?.iter() {
                println!("  {:?} : {}", gopro.kind, gopro.serial);
            }
        },
        _ => {unreachable!()}, // Either no subcommand or one not tested for...
    }

    Ok(())
}
