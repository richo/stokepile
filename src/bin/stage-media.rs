use clap::{App, Arg};
use std::path::PathBuf;

use archiver::cli;
use archiver::config;
use archiver::ctx::Ctx;
use archiver::manual_file::ManualFile;
use archiver::staging;

fn cli_opts<'a, 'b>() -> App<'a, 'b> {
    cli::base_opts()
        .about("Stages media from the local filesystem for the next upload run")
        .arg(Arg::with_name("PATH")
             .help("Path to upload from")
             .required(true)
             .index(1))
        .arg(
            Arg::with_name("preserve")
            .long("preserve")
            .help("Don't remove files after they are staged")
            )
}

fn main() {
    archiver::cli::run(|| {
        let matches = cli_opts().get_matches();

        let cfg = config::Config::from_file(matches.value_of("config").unwrap_or("archiver.toml"));
        let ctx = Ctx::create(cfg?)?;

        let dir = matches.value_of("PATH").expect("Couldn't get path");
        let path = PathBuf::from(dir);
        let device_name = path.file_name()
            .expect("Couldn't get file name")
            .to_str()
            .expect("Couldn't convert device name to str")
            .to_string();

        for file in ManualFile::iter_from(path) {
            staging::stage_file(file, &ctx.staging, &device_name)?;
        }

        Ok(())
    })
}
