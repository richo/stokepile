#![deny(unused_must_use)]

extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate handlebars;
#[macro_use]
extern crate hyper;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

extern crate chrono;
extern crate digest;
extern crate hashing_copy;
extern crate hex;
extern crate libusb;
extern crate ptp;
extern crate regex;
extern crate reqwest;
extern crate sendgrid;
extern crate serde_json;
extern crate sha2;
extern crate toml;
extern crate walkdir;

pub mod dropbox_content_hasher;

pub mod config;
pub mod ctx;
pub mod device;
pub mod dropbox;
mod flysight;
pub mod mailer;
mod mass_storage;
mod peripheral;
pub mod ptp_device;
mod pushover;
pub mod pushover_notifier;
mod reporting;
mod staging;
pub mod storage;
mod version;
pub use version::VERSION;
