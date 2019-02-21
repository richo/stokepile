/// This helper exists as a priviliged bridge to mounting devices.
///
/// It makes a best effort attempt to avoid being actively dangerous, but hasn't been thoroughly
/// audited or proven safe.
use failure::Error;
use serde_json;

use archiver::mountable::MountRequest;

use std::io;


fn main() -> Result<(), Error> {
    let mut stdin = io::stdin();
    let req: MountRequest = serde_json::from_reader(&mut stdin)?;
    println!("{:?}", &req);

    Ok(())
}
