use std::path::PathBuf;
use std::fs;

use clap::{App, Arg};

use archiver::config;
use archiver::ctx::Ctx;
use archiver::mountable;
use archiver::cli;

fn cli_opts<'a, 'b>() -> App<'a, 'b> {
    cli::base_opts()
        .about("Smoke test the mounting infrastructure")
        .arg(Arg::with_name("LABEL")
             .help("Label of the device to test mount")
             .required(true)
             .index(1))
}


fn main() {
    archiver::cli::run(|| {
        let matches = cli_opts().get_matches();

        let cfg = config::Config::from_file(matches.value_of("config").unwrap_or("archiver.toml"));
        let ctx = Ctx::create_without_lock(cfg?)?;

        let mut pb = PathBuf::from("/dev/disk/by-label");
        pb.push(matches.value_of("LABEL").expect("no label"));

        let mp = mountable::UdisksMounter::mount(pb)?;
        for file in fs::read_dir(mp.path())? {
            println!("  {:?}", &file?);
        }

        Ok(())
    });
}
