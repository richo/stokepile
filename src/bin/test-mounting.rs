use std::path::PathBuf;
use std::fs;

use clap::{App, Arg};

use stokepile::config;
use stokepile::ctx::Ctx;
use stokepile::mountable;

fn cli_opts<'a, 'b>(base: App<'a, 'b>) -> App<'a, 'b> {
    base.about("Smoke test the mounting infrastructure")
        .arg(Arg::with_name("LABEL")
             .help("Label of the device to test mount")
             .required(true)
             .index(1))
}


fn main() {
    stokepile::cli::run(cli_opts, |matches| {
        let cfg = config::Config::from_file(matches.value_of("config").unwrap_or("stokepile.toml"));
        let _ctx = Ctx::create_without_lock(cfg?)?;

        let mut pb = PathBuf::from("/dev/disk/by-label");
        pb.push(matches.value_of("LABEL").expect("no label"));

        let mp = mountable::UdisksMounter::mount(pb)?;
        for file in fs::read_dir(mp.path())? {
            println!("  {:?}", &file?);
        }

        Ok(())
    });
}
