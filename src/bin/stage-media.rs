#[macro_use]
extern crate log;

use clap::{App, Arg};
use std::path::PathBuf;

use stokepile::config;
use stokepile::ctx::Ctx;
use stokepile::manual_file::ManualFile;
use stokepile::staging::{Stager, StagedFileExt};
use stokepile::mountable::Mountable;
use stokepile_shared::staging::TrimDetail;

fn cli_opts<'a, 'b>(base: App<'a, 'b>) -> App<'a, 'b> {
    base.about("Stages media from the local filesystem for the next upload run")
        .arg(Arg::with_name("PATH")
            .help("Path to upload from")
            .required(true)
            .index(1))
        .arg(Arg::with_name("preserve")
            .long("preserve")
            .help("Don't erase files after staging them"))
        .arg(Arg::with_name("trim")
            .long("trim")
            .takes_value(true)
            .help("add a trim annotation"))
}

fn main() {
    stokepile::cli::run(cli_opts, |matches| {
        let cfg = config::Config::from_file(matches.value_of("config").unwrap_or("stokepile.toml"));
        let ctx = Ctx::create(cfg?)?;

        let dir = matches.value_of("PATH").expect("Couldn't get path");
        let path = PathBuf::from(dir);
        let device_name = "manual";

        let trim = matches.value_of("trim")
            .map(|trim| {
                let parts: Vec<_> = trim.split(":").collect();
                assert_eq!(parts.len(), 2);
                TrimDetail {
                    start: parts[0].parse().expect("start"),
                    end: parts[1].parse().expect("end"),
                }
            });

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
            let mut file = stager.stage(file, &device_name)?;
            if let Some(ref trim) = trim {
                info!("Adding trim annotation");
                file.add_trim(trim.clone())
                    .expect("add trim");
            }
        }

        Ok(())
    })
}
