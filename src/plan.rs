extern crate chrono;
use chrono::prelude::*;

use std::path::PathBuf;
use std::fmt;

use super::device;
use super::dropbox;
use failure::Error;

use super::ptp_device;

#[derive(Debug)]
enum UploadSource {
    LocalFile(PathBuf),
    PtpFile(ptp_device::GoproFile), // TODO(richo) closure probably?
}

impl UploadSource {
    fn path(&self) -> &str {
        match self {
            UploadSource::LocalFile(path) => path.as_path().to_str().unwrap(),
            UploadSource::PtpFile(file) => file.filename.as_str(),
        }
    }
}


#[derive(Debug)]
pub struct UploadDescriptor {
    local_path: UploadSource,
    capture_time: DateTime<Local>,
    device_name: String,
    extension: String,
}

fn parse_gopro_date(date: &str) -> Result<DateTime<Local>, chrono::ParseError> {
    Local.datetime_from_str(date, "%Y%m%dT%H%M%S")
}

impl UploadDescriptor {
    pub fn remote_path(&self) -> String {
        format!("/{}/{}/{}.{}",
                self.date_component(),
                self.device_name,
                self.time_component(),
                self.extension
                )
    }

    fn date_component(&self) -> String {
        self.capture_time.format("%y-%m-%d").to_string()
    }

    fn time_component(&self) -> String {
        self.capture_time.format("%H-%M-%S").to_string()
    }
}

pub trait ExecutePlan : fmt::Debug {
    fn execute(self: Box<Self>, dropbox::DropboxFilesClient) -> Result<(), Error>;
}

pub struct GoproPlan<'a> {
    name: String,
    connection: ptp_device::GoproConnection<'a>,
    plan: Vec<UploadDescriptor>,
}

impl<'a> fmt::Debug for GoproPlan<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        if fmt.alternate() {
            write!(fmt, "{}:\n", &self.name)?;
            for desc in &self.plan {
                write!(fmt, "{} -> {}\n", desc.local_path.path(), &desc.remote_path())?
            }
            write!(fmt, "")
        } else {
            fmt.debug_struct("GoproPlan")
                .field("connection", &"ptp_device::GoproConnection")
                .field("plan", &self.plan)
                .finish()
        }
    }
}

impl<'a> ExecutePlan for GoproPlan<'a> {
    fn execute(self: Box<Self>, dropbox: dropbox::DropboxFilesClient) -> Result<(), Error> {
        let values = *self;
        let GoproPlan { name: _, mut connection, plan } = values;
        for desc in plan {
            let path = PathBuf::from(&desc.remote_path());
            match desc.local_path {
                UploadSource::PtpFile(gopro_file) => {
                    {
                        let reader = gopro_file.reader(&mut connection);
                        dropbox.upload_from_reader(reader, &path)?;
                    }

                    gopro_file.delete(&mut connection)?;
                }
                UploadSource::LocalFile(_) => {
                    unreachable!();
                }
            }
        }
        Ok(())
    }
}

pub fn create_plan<'a>(device: device::Device<'a>) -> Result<Box<ExecutePlan + 'a>, Error> {
    let mut plan = Vec::new();
    match device {
        device::Device::Gopro(desc, gopro) => {
            let name = desc.name;
            let mut connection = gopro.connect()?;
            for file in connection.files()? {
                let capture_time = parse_gopro_date(&file.capturedate)?;
                plan.push(UploadDescriptor {
                    local_path: UploadSource::PtpFile(file),
                    capture_time: capture_time,
                    device_name: name.clone(),
                    extension: "mp4".to_string(),
                });
            }
            Ok(Box::new(GoproPlan {
                name,
                connection,
                plan,
            }))
        },
        device::Device::MassStorage(_) |
            device::Device::Flysight(_) => {
                unreachable!()
            },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formats_correctly() {
        let datetime = Local.ymd(2017, 11, 22).and_hms(15, 36, 10);
        let path = PathBuf::from("/path/to/whatever");

        let upload = UploadDescriptor {
            local_path: UploadSource::LocalFile(path),
            capture_time: datetime,
            device_name: "test".to_string(),
            extension: "mp4".to_string(),
        };

        assert_eq!(upload.remote_path(), "/17-11-22/test/15-36-10.mp4".to_string());
    }

    #[test]
    fn test_pads_correctly() {
        let datetime = Local.ymd(2001, 1, 2).and_hms(3, 4, 5);
        let path = PathBuf::from("/path/to/whatever");

        let upload = UploadDescriptor {
            local_path: UploadSource::LocalFile(path),
            capture_time: datetime,
            device_name: "test".to_string(),
            extension: "mp4".to_string(),
        };

        assert_eq!(upload.remote_path(), "/01-01-02/test/03-04-05.mp4".to_string());
    }

    #[test]
    fn test_parses_gopro_date_correctly() {
        let dt = Local.ymd(2015, 1, 1).and_hms(0, 6, 49);
        // TODO(richo) get better testcases
        assert_eq!(parse_gopro_date("20150101T000649"),
                   Ok(dt.clone()));
    }
}
