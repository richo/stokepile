#[macro_use]
extern crate log;

use clap::App;

use stokepile::cli;
use stokepile::config;
use stokepile::ctx::Ctx;
use stokepile::staging::{StagingLocation, StagedFileExt};
use stokepile::mountable::Mountable;

fn cli_opts<'a, 'b>() -> App<'a, 'b> {
    cli::base_opts()
        .about("Applies any pending transforms on the staged media")
}

fn main() {
    stokepile::cli::run(|| {
        let matches = cli_opts().get_matches();

        let cfg = config::Config::from_file(matches.value_of("config").unwrap_or("stokepile.toml"));
        let ctx = Ctx::create(cfg?)?;

        let staging = ctx.staging().mount()?;
        info!("Staging to: {:?}", &staging);

        for file in staging.staged_files()? {
            file.apply_trim().expect("apply_transforms");
        }

        Ok(())
    })
}
