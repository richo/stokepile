use std::collections::HashMap;

use crate::staging::UploadDescriptor;

use failure::Error;
use handlebars::{Handlebars, TemplateRenderError};
use serde::ser::{Serialize, Serializer, SerializeStruct};

handlebars_helper!(header: |v: str| format!("{}\n{}", v, str::repeat("=", v.len())));

fn handlebars() -> Handlebars {
    let mut handlebars = Handlebars::new();
    handlebars.register_helper("header", Box::new(header));
    handlebars.set_strict_mode(true);
    handlebars
}

#[derive(Debug)]
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

/// A report describing how a series of upload transactions went.
#[derive(Debug, Default, Serialize)]
pub struct UploadReport {
    files: HashMap<String, Vec<ReportEntry>>,
}

/// An entry in the report.
///
/// results is a Vec of service-name, status tuples.
#[derive(Debug, Serialize)]
pub struct ReportEntry {
    #[serde(serialize_with = "format_report")]
    desc: UploadDescriptor,
    results: Vec<(String, UploadStatus)>,
}

// We serialize with a custom serializer here, in order to use our date representation in the
// reports.
//
// This naively seems like it'd be easier to implement on the UploadDescriptor, but it's
// `Serialize` impl is responsible for making sure it round trips the disc safely.
fn format_report<S>(desc: &UploadDescriptor, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer,
{
    let mut ser = serializer.serialize_struct("UploadDescriptor", 3)?;
    ser.serialize_field("capture_time", &desc.capture_time.format("%Y-%m-%d %H:%M:%S").to_string())?;
    ser.serialize_field("extension", &desc.extension)?;
    ser.serialize_field("size", &desc.size)?;
    ser.end()

}

impl ReportEntry {
    /// Bind an UploadDescriptor to this entry, returning the finalised ReportEntry.
    pub fn new(desc: UploadDescriptor, results: Vec<(String, UploadStatus)>) -> ReportEntry {
        ReportEntry { desc, results }
    }
}

impl ReportEntry {
    /// Was every attempt to upload in this transaction successful.
    pub fn is_success(&self) -> bool {
        self.results.iter().all(|r| match r.1 {
            UploadStatus::AlreadyUploaded | UploadStatus::Succeeded => true,
            UploadStatus::Errored(_) => false,
        })
    }
}

impl UploadReport {
    /// Attach a ReportEntry to this report.
    pub fn record_activity(&mut self, entry: ReportEntry) {
        let uploads = self
            .files
            .entry(entry.desc.device_name.clone())
            .or_insert_with(|| vec![]);
        uploads.push(entry);
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
        report.record_activity(ReportEntry::new(
            UploadDescriptor {
                capture_time: Local.ymd(2018, 8, 24).and_hms(9, 55, 30),
                device_name: "test-device".to_string(),
                extension: "mp4".to_string(),
                content_hash: [66; 32],
                size: 0,
            },
            vec![
                ("vimeo".into(), UploadStatus::Succeeded),
                ("youtube".into(), UploadStatus::Succeeded),
            ],
        ));
        report.record_activity(ReportEntry::new(
            UploadDescriptor {
                capture_time: Local.ymd(2018, 8, 24).and_hms(12, 30, 30),
                device_name: "test-device".to_string(),
                extension: "mp4".to_string(),
                content_hash: [66; 32],
                size: 0,
            },
            vec![
                (
                    "vimeo".into(),
                     UploadStatus::Succeeded
                ),
                (
                    "youtube".into(),
                    UploadStatus::Errored(format_err!("Something bad happened")),
                ),
            ],
        ));
        report
    }

    #[test]
    fn test_renders_report() {
        let report = dummy_report();
        // We use LocalTime throughout, since it's reasonable to assume that is correct. However,
        // localtime formats including its offset, which we can't predict in tests. We construct
        // one, remove its offset, and template it in here for the testcase.
        let expected = format!(
            "\
ARCHIVER UPLOAD REPORT
======================

test-device
===========

    2018-08-24 09:55:30.mp4 (0b)
    # vimeo: Succeeded
    # youtube: Succeeded

    2018-08-24 12:30:30.mp4 (0b)
    # vimeo: Succeeded
    # youtube: Upload failed: ErrorMessage {{ msg: &quot;Something bad happened&quot; }}
");
        assert_eq!(report.to_plaintext().unwrap(), expected);
    }
}

static UPLOAD_REPORT_TEMPLATE: &'static str = "\
{{header \"ARCHIVER UPLOAD REPORT\"}}

{{#each files}}{{header @key}}
{{#each this}}
    {{this.desc.capture_time}}.{{this.desc.extension}} ({{this.desc.size}}b)
{{#each this.results}}    # {{this.[0]}}: {{this.[1]}}
{{/each}}\
{{/each}}{{/each}}\
";
