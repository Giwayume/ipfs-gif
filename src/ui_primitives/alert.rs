use askama::Template;

#[derive(Template)]
#[template(path = "ui_primitives/alert.html")]
pub struct AlertTemplate<'a> {
    pub variant: &'a str,
    pub message_html: String,
}

fn get_icon_class(variant: &str) -> &str {
    match variant {
        "success" => "bi-check-circle-fill",
        "danger" => "bi-exclamation-octagon-fill",
        _ => "bi-info-circle-fill",
    }
}