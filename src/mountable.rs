use std::path::{Path, PathBuf};
use std::io::Write;
use std::process::{Command, Stdio};

use failure::Error;
use serde_json;

#[derive(Debug, Serialize, Deserialize)]
pub struct MountRequest {
    device: PathBuf,
    mountpoint: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MountResponse {
    Success,
    // TODO(richo) bring more structural data about the error
    Failure(String),
}

impl MountRequest {
    pub fn process(self) -> Result<(), Error> {

        Ok(())
    }
}

#[derive(Debug)]
pub struct CmdMounter {
}

impl CmdMounter {
    pub fn mount<U, T>(device: U, mountpoint: T) -> Result<MountResponse, Error>
    where U: AsRef<Path>,
          T: AsRef<Path>
    {
        // TODO(richo) Find the helper properly.
        let child = Command::new("mount")
            .arg(device.as_ref())
            .arg(mountpoint.as_ref())
            .spawn()?;

        let ret = child.wait_with_output()?;
        if ret.status.success() {
            Ok(MountResponse::Success)
        } else {
            Ok(MountResponse::Failure(String::from_utf8(ret.stderr)?))
        }
    }
}

// pub struct HelperMounter {
// }

// impl HelperMounter {
//     pub fn mount<U, T>(device: U, mountpoint: T) -> Result<MountResponse, Error>
//     where U: AsRef<Path>,
//           T: AsRef<Path>
//     {
//         // TODO(richo) Find the helper properly.
//         let mut child = Command::new("mount-helper")
//             .stdin(Stdio::piped())
//             .stdout(Stdio::piped())
//             .spawn()?;

//         {
//             let json = serde_json::to_string(&self)?;
//             let stdin = child.stdin.as_mut().expect("Couldn't get child stdio");
//             stdin.write_all(json.as_bytes())?;
//         }

//         let output = child.wait_with_output()?;
//         println!("OUTPUTOUTPUT: {:?}", &output);
//         Ok(())
//     }
// }

pub trait Mountable {
    type Mountpoint: AsRef<Path>;

    fn device(&self) -> &Path;

    fn set_mountpoint(&mut self, mountpoint: Self::Mountpoint);

    fn mount(&mut self, mountpoint: Self::Mountpoint) -> Result<(), Error> {
        let req = MountRequest {
            device: self.device().into(),
            mountpoint: mountpoint.as_ref().into(),
        };

        req.process()?;

        self.set_mountpoint(mountpoint);
        Ok(())
    }
}
