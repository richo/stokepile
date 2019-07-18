#[macro_use]
extern crate log;

use clap::{App, Arg};
use std::path::PathBuf;

use stokepile::cli;
use stokepile::config;
use stokepile::ctx::Ctx;
use stokepile::manual_file::ManualFile;
use stokepile::staging::Stager;
use stokepile::mountable::Mountable;

fn cli_opts<'a, 'b>() -> App<'a, 'b> {
    cli::base_opts()
        .about("Stages media from the local filesystem for the next upload run")
        .arg(Arg::with_name("PATH")
             .help("Path to upload from")
             .required(true)
             .index(1))
        .arg(Arg::with_name("preserve")
             .long("preserve")
             .help("Don't erase files after staging them"))
}

fn main() {
    stokepile::cli::run(|| {
        let matches = cli_opts().get_matches();

        let cfg = config::Config::from_file(matches.value_of("config").unwrap_or("stokepile.toml"));
        let ctx = Ctx::create(cfg?)?;

        let dir = matches.value_of("PATH").expect("Couldn't get path");
        let path = PathBuf::from(dir);
        let device_name = path.file_name()
            .expect("Couldn't get file name")
            .to_str()
            .expect("Couldn't convert device name to str")
            .to_string();

        let staging = ctx.staging().mount()?;
        info!("Staging to: {:?}", &staging);

        let stager = match matches.is_present("preserve") {
            true => {
                info!("Preserving input files");
                Stager::preserving(staging)
            },
            false => {
                warn!("Will remove input files after staging");
                Stager::destructive(staging)
            }
        };


        for file in ManualFile::iter_from(path) {
            stager.stage(file, &device_name)?;
        }

        Ok(())
    })
}
