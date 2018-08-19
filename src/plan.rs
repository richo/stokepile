use std::collections::HashMap;

use super::config::Peripheral;

pub struct UploadDescriptor;

pub struct UploadPlan {
    //            day             device      content
    plan: HashMap<String, HashMap<String, Vec<UploadDescriptor>>>,
}

pub struct LogicalDay {
}

impl LogicalDay {
    fn as_upload_format(&self) -> String {
        "01-01-2018".to_string()
    }

    fn files(&self) -> Vec<LogicalFile> {
        vec![]
    }
}

pub struct LogicalFile {
}

impl LogicalFile {
}

impl UploadPlan {
    pub fn new() -> UploadPlan {
        UploadPlan {
            plan: HashMap::new(),
        }
    }

    pub fn from_peripheral(&mut self, peripheral: Box<Peripheral>) {
        for day in peripheral.days() {
            let mut mapped_day = self.plan.entry(day.as_upload_format()).or_insert_with(|| HashMap::new());
            let mut device = mapped_day.entry(peripheral.name().clone()).or_insert_with(|| Vec::new());
            for file in day.files() {
                device.push(UploadDescriptor {
                })
            }
        }
    }

    pub fn execute(self) {
    }
}
