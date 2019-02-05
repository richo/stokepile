use crate::{AUTHOR, VERSION};

use clap::{App, Arg};
use dirs;
use lockfile;
use failure::{format_err, Error};

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

/// Acquire the lock for running mutative archiver operations, or return the error associated with
/// the attempt (Most likely meaning that the lock is already held).
pub fn acquire_lock() -> Result<lockfile::Lockfile, Error> {
    // TODO(richo) for now we just stash this in the user's home directory.
    let home = dirs::home_dir().ok_or(format_err!("Couldn't open HOME"))?;
    let lock_path = home.join(".archiver.lock");
    Ok(lockfile::Lockfile::create(lock_path)?)
}

/// Setup logging for archiver. This sets the log level to INFO if unset and configures the logging
/// facade favoured by archiver's clis.
pub fn init_logging() {
    if let None = ::std::env::var_os("RUST_LOG") {
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
        if ::std::env::var("RUST_BACKTRACE").is_ok() {
            error!("{:?}", e.backtrace());
        }
        ::std::process::exit(1);
    }
}
