#[macro_use]
extern crate serde_derive;

extern crate clap;
use clap::App;

mod version;
mod config;

fn main() {
    App::new("archiver")
        .version(version::VERSION)
        .about("Footage archiver")
        .author("richö butts")
        .get_matches();
}
