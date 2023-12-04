#![deny(unused_must_use, missing_debug_implementations)]
#![warn(rust_2018_idioms)]
#![warn(clippy::all)]

#[macro_use]
extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
#[macro_use]
extern crate redacted_debug;

#[cfg(all(test, feature = "web"))]
macro_rules! client_for_routes {
    (config: $($route:ident),+ => $client:ident) => {
        fn $client() -> rocket::local::Client {
            let routes = routes![
                // We always implicitly allow signin since there isn't currently another way to get
                // an authenticated session
                crate::web::routes::sessions::post_signin,
                crate::web::routes::sessions::signin_json,

                $($route),+
            ];
            let rocket = crate::web::configure_rocket(routes)
                .manage(crate::web::db::init_pool(true));
            rocket::local::Client::new(rocket).expect("valid rocket instance")
        }
    };
    (media: $($route:ident),+ => $client:ident) => {
        fn $client() -> rocket::local::Client {
            let staging = crate::staging::MountedStaging::from(crate::test_helpers::tempdir());
            let routes = routes![
                $($route),+
            ];
            let rocket = crate::web::configure_rocket(routes)
                .manage(staging);

            rocket::local::Client::new(rocket).expect("valid rocket instance")
        }
    };
}

/// Transition module for async. Always used unqualified to make grepping easier later;
pub mod async_hacks;

/// A client to the web interface.
pub mod client;

/// Details pertaining to parsing the configuration file, as well as constructing the internal
/// objects specified by the configuration.
pub mod config;

/// The global context object that is threaded throughout the run of the program. This module also
/// deals with some implementation details, like ensuring that the staging directory exists as part
/// of standing up the context.
pub mod ctx;

/// Some helpers associated with driving the clis that ship with stokepile.
pub mod cli;

/// Some helpers to abstract over the various types of devices that we can interact with. Much of
/// this will probably go away at some point, and Device will become a trait instead of the enum
/// that it is today.
///
/// This module also contains the logic for simply enumerating all currently attached devices as
/// part of generating a plan for an upload run.
pub mod device;

/// A drop in replacement for libusb for use in contexts where we can't actually link against
/// libusb (eg, the web server).
///
/// Attempting to invoke any actual libusb stuff panics.
pub mod dummy_libusb;

/// A drop in replacement for ptp for use in contexts where we can't actually link against
/// libusb (eg, the web server).
///
/// Attempting to invoke any actual ptp stuff panics.
pub mod dummy_ptp;

/// Our interface to the dropbox API. This should really be it's own crate, but until I have the
/// enthusiasm to implement more than the bare minimum stokepile needs, it will remain vendored
/// here.
pub mod dropbox;

/// Flysight specific code. This mostly relates to parsing out the filenames that flysights create.
mod flysight;

/// A module concerning itself with presenting information in a human readable format.
pub mod formatting;

/// A storage adaptor governing a local storage device to archive the data onto.
pub mod local_backup;

/// Contains the MailReport trait which all mailers must implement, as well as the stokepile
/// specific glue for the `SGClient` object we use from the `sendgrid` crate.
pub mod mailer;

/// A struct useful for when you want to stage files more arbitrarily than those owned by a
/// particular device.
pub mod manual_file;

/// Code relating to the `mass_storage` device type. This is any device that can be mounted to the
/// local filesystem.
mod mass_storage;

/// Message types used for communication between the server and client components.
pub mod messages;

/// Contains machinery relating to mounting and unmounting devices.
pub mod mountable;

/// Our bindings to the ptp crate, which we use to talk to devices like Gopros over USB, allowing
/// us to avoid having to pull the SD card in order to upload footage.
pub mod ptp_device;

/// Contains the `Notify` trait, which all notifiers must implement. Contains impls, as well as a
/// little local glue to bind `config` and `pushover` together.
pub mod pushover_notifier;

/// Contains the machinery for generating an upload report. This handles both building the report
/// object up in memory, as well as rendering it to something we can mail to a user.
mod reporting;

/// Machinry for locally staging files from attached devices. It includes the `Staging` trait,
/// which when implemented allows for not implementing some of the heavy lifting.
pub mod staging;

/// Contains the logic for consuming the locally staged files and uploading them to the selected
/// storage backend. Also deals with deduping (Locally hashing files to ensure that we're not
/// pointlessly uploading things that are already there) and cleaning up the local staging area.
pub mod storage;

/// The trimmer transform, which allows for trimming media before uploading it.
pub mod trimmer;

/// The vimeo upload backend.
pub mod vimeo;

mod version;

/// A notifier that pushes notifications out via the web service.
mod web_notifier;

/// What version of stokepile do you have :)
pub use crate::version::VERSION;

/// Who wrote this mess
pub use crate::version::AUTHOR;

/// Potentially, what git sha is this.
pub use crate::version::GIT_HASH;

#[cfg(test)]
/// Helpers for use in tests
mod test_helpers;

#[cfg(test)]
extern crate filetime;

#[cfg(feature = "web")]
#[macro_use]
extern crate rocket;
#[cfg(feature = "web")]
pub mod web;
#[cfg(feature = "web")]
#[macro_use]
extern crate diesel;
#[cfg(feature = "web")]
#[macro_use]
extern crate diesel_derive_enum;
#[cfg(feature = "web")]
extern crate bcrypt;
#[cfg(feature = "web")]
extern crate rand;
