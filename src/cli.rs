use crate::{AUTHOR, VERSION};

use std::io::Read;
use clap::{App, Arg};
use dotenv;

/// Create the base set of clap options common to all cli commands
pub fn base_opts<'a, 'b>() -> App<'a, 'b> {
    App::new("archiver")
        .version(VERSION)
        .author(AUTHOR)
        .arg(
            Arg::with_name("config")
                .long("config")
                .takes_value(true)
                .help("Path to configuration file"),
        )
}

/// Setup logging for archiver. This sets the log level to INFO if unset and configures the logging
/// facade favoured by archiver's clis.
pub fn init_logging() {
    if ::std::env::var_os("RUST_LOG").is_none() {
        ::std::env::set_var("RUST_LOG", "INFO");
    }
    pretty_env_logger::init();
}

/// Run a given closure with logging configured, and deal with any errors. This allows you to have
/// a fairly simple main, eg:
///
/// ```
/// use archiver::cli::run;
///
/// fn main() {
///     run(|| {
///         // Do stuff here, including using the ? operator with reckless abandon.
///         // ...
///         // You must however return Ok(())
///         Ok(())
///     });
/// }
/// ```
///
/// Even in our post Termination world, this is valuable in order to assert that logging is setup,
/// and in future that we have a panic handler.
pub fn run(main: fn() -> Result<(), ::failure::Error>) {
    init_logging();
    if let Err(e) = main() {
        error!("Error running archiver");
        error!("{:?}", e);
        if ::std::env::var("ARCHIVER_BACKTRACE").is_ok() {
            error!("{:?}", e.backtrace());
        } else {
            info!("Set ARCHIVER_BACKTRACE for more information");
        }
        ::std::process::exit(1);
    }
}

pub fn run_and_wait(main: fn() -> Result<(), ::failure::Error>) {
    init_logging();
    if let Err(e) = main() {
        error!("Error running archiver");
        error!("{:?}", e);
        if ::std::env::var("ARCHIVER_BACKTRACE").is_ok() {
            error!("{:?}", e.backtrace());
        } else {
            info!("Set ARCHIVER_BACKTRACE for more information");
        }
    }
    info!("Finished! Press return to exit.");
    let mut buf = [0; 0];
    std::io::stdin().read(&mut buf).expect("Couldn't read from stdin");
}


/// Configure dotenv, ignoring any errors purely because it can't find the dotenv file.
pub fn init_dotenv() -> Result<(), dotenv::Error> {
    match dotenv::dotenv() {
        Err(dotenv::Error::Io(_)) |
        Ok(_) => Ok(()),
        Err(e) => Err(e)?,
    }
}
