use std::collections::HashMap;

use staging::UploadDescriptor;

use failure::Error;
use handlebars::{Handlebars, TemplateRenderError};
use serde::ser::{Serialize, Serializer};

handlebars_helper!(header: |v: str| format!("{}\n{}", v, str::repeat("=", v.len())));

fn handlebars() -> Handlebars {
    let mut handlebars = Handlebars::new();
    handlebars.register_helper("header", Box::new(header));
    handlebars.set_strict_mode(true);
    handlebars
}

pub enum UploadStatus {
    AlreadyUploaded,
    Succeeded,
    Errored(Error),
}

impl Serialize for UploadStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let msg = match self {
            UploadStatus::AlreadyUploaded => "Already uploaded".to_string(),
            UploadStatus::Succeeded => "Succeeded".to_string(),
            UploadStatus::Errored(error) => format!("Upload failed: {:?}", error),
        };
        serializer.serialize_str(&msg)
    }
}

#[derive(Default, Serialize)]
pub struct UploadReport {
    files: HashMap<String, Vec<(UploadStatus, UploadDescriptor)>>,
}

impl UploadReport {
    pub fn record_activity(&mut self, status: UploadStatus, desc: UploadDescriptor) {
        let uploads = self
            .files
            .entry(desc.device_name.clone())
            .or_insert_with(|| vec![]);
        uploads.push((status, desc))
    }

    pub fn to_plaintext(&self) -> Result<String, TemplateRenderError> {
        handlebars().render_template(UPLOAD_REPORT_TEMPLATE, &self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::prelude::*;

    fn dummy_report() -> UploadReport {
        let mut report: UploadReport = Default::default();
        report.record_activity(
            UploadStatus::Succeeded,
            UploadDescriptor {
                capture_time: Local.ymd(2018, 8, 24).and_hms(9, 55, 30),
                device_name: "test-device".to_string(),
                extension: "mp4".to_string(),
                content_hash: [66; 32],
                size: 0,
            },
        );
        report.record_activity(
            UploadStatus::Errored(format_err!("Something bad happened")),
            UploadDescriptor {
                capture_time: Local.ymd(2018, 8, 24).and_hms(12, 30, 30),
                device_name: "test-device".to_string(),
                extension: "mp4".to_string(),
                content_hash: [66; 32],
                size: 0,
            },
        );
        report
    }

    #[test]
    fn test_renders_report() {
        let report = dummy_report();
        assert_eq!(
            &report.to_plaintext().unwrap(),
            "\
ARCHIVER UPLOAD REPORT
======================

test-device
===========

    # Succeeded
2018-08-24T09:55:30-07:00.mp4 (0b)

    # Upload failed: ErrorMessage { msg: &quot;Something bad happened&quot; }
2018-08-24T12:30:30-07:00.mp4 (0b)
"
        );
    }
}

static UPLOAD_REPORT_TEMPLATE: &'static str = "\
{{header \"ARCHIVER UPLOAD REPORT\"}}

{{#each files}}{{header @key}}
{{#each this}}
    # {{this.[0]}}
{{this.[1].capture_time}}.{{this.[1].extension}} ({{this.[1].size}}b)
{{/each}}{{/each}}\
";
