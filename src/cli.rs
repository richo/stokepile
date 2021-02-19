use crate::{GIT_HASH, AUTHOR, VERSION};

use std::io::Read;
use clap::{App, Arg, ArgMatches};
use dotenv;

lazy_static! {
    static ref VERSION_STRING: String = if let Some(hash) = GIT_HASH {
        format!("{} - {}", VERSION, hash)
    } else {
        VERSION.into()
    };
}

/// Create the base set of clap options common to all cli commands
fn base_opts<'a, 'b>() -> App<'a, 'b> {
    App::new("stokepile")
        .author(AUTHOR)
        .version(&**VERSION_STRING)
        .arg(
            Arg::with_name("config")
                .long("config")
                .takes_value(true)
                .help("Path to configuration file"),
        )
}

/// Setup logging for stokepile. This sets the log level to INFO if unset and configures the logging
/// facade favoured by stokepile's clis.
fn init_logging() {
    if ::std::env::var_os("RUST_LOG").is_none() {
        ::std::env::set_var("RUST_LOG", "INFO");
    }
    pretty_env_logger::init();
}

/// Run a given closure with logging configured, and the result of parsing cli arguments, and deal
/// with any errors. This allows you to have a fairly simple main. Run accepts a function to
/// configure the clap cli, you can just pass it back without consequence.
///
/// ```
/// use stokepile::cli::run;
///
/// fn main() {
///     run(|clap| clap, |_matches| {
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
pub fn run<'a, 'b>(setup_cli: fn(App<'a, 'b>) -> App<'a, 'b>, main: fn(ArgMatches<'_>) -> Result<(), ::failure::Error>) {
    init_logging();
    init_dotenv()
        .expect("Couldn't init dotenv");
    let matches = setup_cli(base_opts()).get_matches();
    if let Err(e) = main(matches) {
        error!("Error running stokepile");
        error!("{:?}", e);
        if ::std::env::var("STOKEPILE_BACKTRACE").is_ok() {
            error!("Backtrace information:");
            error!("{:?}", e.backtrace());
        } else {
            info!("Set STOKEPILE_BACKTRACE for more information");
        }
        ::std::process::exit(1);
    }
}

pub fn run_and_wait(main: fn() -> Result<(), ::failure::Error>) {
    init_logging();
    if let Err(e) = main() {
        error!("Error running stokepile");
        // TODO(richo) Figure out how to unify without RUST_BACKTRACE, and not double print the
        // error.
        error!("{:?}", e);
        if ::std::env::var("STOKEPILE_BACKTRACE").is_ok() {
            error!("{:?}", e.backtrace());
        } else {
            info!("Set STOKEPILE_BACKTRACE for more information");
        }
    }
    info!("Finished! Press return to exit.");
    let mut buf = [0; 0];
    std::io::stdin().read(&mut buf).expect("Couldn't read from stdin");
}


/// Configure dotenv, ignoring any errors purely because it can't find the dotenv file.
fn init_dotenv() -> Result<(), dotenv::Error> {
    match dotenv::dotenv() {
        Err(dotenv::Error::Io(_)) |
        Ok(_) => Ok(()),
        Err(e) => Err(e)?,
    }
}
