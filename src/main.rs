extern crate clap;
use clap::App;

mod version;

fn main() {
    App::new("archiver")
        .version(version::VERSION)
        .about("Footage archiver")
        .author("richö butts")
        .get_matches();
}
