#[macro_use] extern crate serde_derive;
#[macro_use] extern crate failure;
#[macro_use] extern crate lazy_static;

use std::process;

extern crate clap;
use clap::{App,SubCommand,Arg};

extern crate libusb;

use failure::Error;

mod config;
mod ctx;
mod device;
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
    let matches = cli_opts().get_matches();

    // TODO(richo) run -> Result<(), Error> so I can use ?
    let ctx = match create_ctx(&matches) {
        Ok(cfg) => cfg,
        Err(e) => {
            println!("Error initializing archiver");
            println!("{}", e);
            process::exit(1);
        },
    };

    match matches.subcommand() {
        ("daemon", Some(subm))  => {
            unimplemented!();
        },
        ("run", Some(subm)) => {
            let mut plan = plan::UploadPlan::new();
            // Figure out which cameras we're gunna be operating on
            let devices = device::attached_devices(&ctx);
            println!("{:?}", plan);
            plan.execute();
        },

        ("scan", Some(subm)) => {
            println!("{:#?}", ptp_device::locate_gopros(&ctx));
        },
        _ => {unreachable!()}, // Either no subcommand or one not tested for...
    }
}
