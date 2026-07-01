use garde::{ Report };

pub fn report_has_field(report: &Report, field_name: &str) -> bool {
    for (path, _) in report.iter() {
        if path.to_string() == field_name {
            return true
        }
    }
    false
}

pub fn create_simple_report(path: String, error_message: String) -> Report {
    let mut report = Report::new();
    let path = garde::Path::new(path);
    let error = garde::Error::new(error_message);
    report.append(path, error);
    report
}
