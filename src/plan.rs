use std::path::PathBuf;

use super::config::Peripheral;

#[derive(Debug)]
pub struct UploadDescriptor {
    local_path: PathBuf,
    remote_path: PathBuf,
}

#[derive(Debug)]
pub struct UploadPlan {
    plan: Vec<UploadDescriptor>,
}

pub struct LogicalFile {
    local_path: PathBuf,
    device_name: String,
}

impl LogicalFile {
    pub fn upload_formatted_date(&self) -> String {
        // TODO(richo) Come up with a plan for timezones?
        "01-01-2018".to_string()
    }

    pub fn remote_path(&self) -> String {
        format!("{}/{}/{}",
                self.upload_formatted_date(),
                self.device_name,
                self.file_basename(),
                )
    }

    pub fn file_basename(&self) -> &str {
        self.local_path.file_name().unwrap().to_str().unwrap()
    }
}

impl UploadPlan {
    pub fn new() -> UploadPlan {
        UploadPlan {
            plan: Vec::new(),
        }
    }

    pub fn from_peripheral(&mut self, peripheral: Box<Peripheral>) {
        for file in peripheral.files() {
            self.plan.push(UploadDescriptor {
                local_path: file.local_path.clone(),
                remote_path: PathBuf::from(file.remote_path()),
            })
        }
    }

    pub fn execute(self) {
    }
}
