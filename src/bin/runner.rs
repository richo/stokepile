#[macro_use]
extern crate log;



use pretty_env_logger;


use rpassword;

use clap::{App, Arg, SubCommand};
use failure::Error;
use std::env;
use std::fs::File;
use std::io::{self, Write};
use std::process;
use std::thread;

use archiver::client;
use archiver::config;
use archiver::ctx::Ctx;
use archiver::device;
use archiver::mailer::MailReport;
use archiver::ptp_device;
use archiver::pushover_notifier::Notify;
use archiver::storage;
use archiver::{AUTHOR, VERSION};

fn cli_opts<'a, 'b>() -> App<'a, 'b> {
    App::new("archiver")
        .version(VERSION)
        .about("Footage archiver")
        .author(AUTHOR)
        .arg(
            Arg::with_name("config")
                .long("config")
                .takes_value(true)
                .help("Path to configuration file"),
        )
        .subcommand(
            SubCommand::with_name("daemon")
                .version(VERSION)
                .author(AUTHOR)
                .about("Runs archiver in persistent mode"),
        )
        .subcommand(
            SubCommand::with_name("scan")
                .version(VERSION)
                .author("richö butts")
                .about("Scan for attached devices"),
        )
        .subcommand(
            SubCommand::with_name("login")
                .version(VERSION)
                .author(AUTHOR)
                .about("Login to archiver web for config fetching"),
        )
        .subcommand(
            SubCommand::with_name("fetch")
                .version(VERSION)
                .author(AUTHOR)
                .about("Fetch config from upstream, overwriting whatever you have"),
        )
        .subcommand(
            SubCommand::with_name("run")
                .about("Runs archiver in persistent mode")
                .version(VERSION)
                .author(AUTHOR)
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

    let cfg = config::Config::from_file(matches.value_of("config").unwrap_or("archiver.toml"));

    match matches.subcommand() {
        ("daemon", Some(_subm)) => {
            unimplemented!();
        }
        ("run", Some(_subm)) => {
            let ctx = Ctx::create(cfg?)?;
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

            // Let the cameras populate the plan
            for device in devices {
                let msg = format!("Finished staging: {}", device.name());
                device.stage_files(&ctx.staging)?;
                ctx.notifier.notify(&msg)?;
            }
            ctx.notifier.notify("Finished staging media")?;

            // Run the uploader thread syncronously as a smoketest for the daemon mode
            let staging = ctx.staging.clone();
            let report = thread::spawn(move || storage::upload_from_staged(&staging, &backends))
                .join()
                .expect("Upload thread panicked")?;
            ctx.notifier.notify("Finished uploading media")?;
            let plaintext = report.to_plaintext()?;
            println!("{}", plaintext);
            ctx.mailer.send_report(&plaintext)?;
        }
        ("scan", Some(_subm)) => {
            let ctx = Ctx::create(cfg?)?;
            println!("Found the following gopros:");
            for gopro in ptp_device::locate_gopros(&ctx)?.iter() {
                println!("  {:?} : {}", gopro.kind, gopro.serial);
            }
        }
        ("fetch", Some(_subm)) => {
            let base = match cfg {
                Ok(cfg) => {
                    cfg.api_base().to_string()
                },
                Err(_) => {
                    info!("Error loading config, proceeding with default api base");
                    config::DEFAULT_API_BASE.to_string()
                },
            };
            let token = config::AccessToken::load()?;
            info!("Creating client");
            let client = client::ArchiverClient::new(&base)?;
            info!("Fetching config from {}", &base);
            let config = client.fetch_config(token)?;
            let filename = matches.value_of("config").unwrap_or("archiver.toml");
            let mut fh = File::create(&filename)?;
            fh.write(config.to_toml().as_bytes())?;
            info!("Wrote config to {}", &filename);
        }
        // Login to upstream, adding the token to your local config file
        ("login", Some(_subm)) => {
            let base = match cfg {
                Ok(cfg) => {
                    cfg.api_base().to_string()
                },
                Err(_) => {
                    info!("Error loading config, proceeding with default api base");
                    config::DEFAULT_API_BASE.to_string()
                },
            };
            let client = client::ArchiverClient::new(&base)?;
            let mut email = String::new();
            let stdin = io::stdin();
            let password;
            println!("Logging into {}", base);
            print!("email: ");
            io::stdout().flush()?;
            stdin.read_line(&mut email)?;
            password = rpassword::prompt_password_stdout("password: ")?;
            println!("Logging in");
            let token = client.login(email.trim_right(), &password)?;
            println!("Token recieved, saving to ~/{}", config::TOKEN_FILE_NAME);

            // TODO(richo) rewrite config including token
            config::AccessToken::save(&token)?;
        }
        _ => {
            error!("No subcommand provided");
            unreachable!();
        } // Either no subcommand or one not tested for...
    }

    Ok(())
}
