#[macro_use]
extern crate log;

use clap::{App, Arg};

use stokepile::cli;
use stokepile::metadata_extractor;

fn cli_opts<'a, 'b>() -> App<'a, 'b> {
    cli::base_opts()
        .about("Attempts to figure out where the freefall is in a GoPro video with telemetry")
        .arg(
            Arg::with_name("path")
            .long("path")
            .help("Path to the gopro video")
            .required(true)
            .takes_value(true)
            )
}

fn main() {
    stokepile::cli::run(|| {
        let matches = cli_opts().get_matches();
        let path = matches.value_of("path").unwrap();
        let meta = metadata_extractor::metadata(&path)?;
        let messages = meta.parse_as_gopro()?;
        info!("{:#?}", &messages);
        Ok(())
    })
}
