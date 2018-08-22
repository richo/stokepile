extern crate chrono;
use chrono::prelude::*;

use std::path::PathBuf;

use super::device;

#[derive(Debug)]
enum UploadSource {
    LocalFile(PathBuf),
    PtpDevice(), // TODO(richo) closure probably?
}

#[derive(Debug)]
pub struct UploadDescriptor {
    local_path: UploadSource,
    capture_time: DateTime<Local>,
    device_name: String,
    extension: String,
    // TODO store the data in an efficient manner
    // Especially for PTP where we might accidentally materialise the full storage of a gopro
}

impl UploadDescriptor {
    pub fn remote_path(&self) -> String {
        format!("{}/{}/{}.{}",
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

#[derive(Debug)]
pub struct UploadPlan {
    plan: Vec<UploadDescriptor>,
}

impl UploadPlan {
    pub fn new() -> UploadPlan {
        UploadPlan {
            plan: Vec::new(),
        }
    }

    /// Interrogates the device, populating the plan
    pub fn update(&mut self, device: device::Device) {
        match device {
            device::Device::Gopro(desc, mut gopro) => {
                for file in gopro.files() {
                    println!("file: {:?}", file);
                }
            },
            device::Device::MassStorage(_) |
            device::Device::Flysight(_) => {
            },
        }
    }

    pub fn execute(self) {
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

        assert_eq!(upload.remote_path(), "17-11-22/test/15-36-10.mp4".to_string());
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

        assert_eq!(upload.remote_path(), "01-01-02/test/03-04-05.mp4".to_string());
    }
}
