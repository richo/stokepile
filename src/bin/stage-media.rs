use clap::{App, Arg};

use archiver::cli;
use archiver::config;
use archiver::ctx::Ctx;
use archiver::manual_file::ManualFile;
use archiver::staging;

use walkdir;

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

        let path = matches.value_of("PATH").expect("Couldn't get path");

        for entry in walkdir::WalkDir::new(path).into_iter() {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue
            }

            let fh = ManualFile::from_path(entry.path())?;
            staging::stage_file(fh, &ctx.staging, "manual")?;
        }

        Ok(())
    })
}
