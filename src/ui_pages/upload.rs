use std::error::Error;
use askama::Template;
use garde::{ Report };

use crate::router::routes::upload::UploadPageContext;
use crate::router::validation::report_has_field;
use crate::ui_primitives::alert::AlertTemplate;

#[derive(Template)]
#[template(path = "ui_pages/upload.html", blocks = ["page_content", "upload_tags"])]
pub struct UploadTemplate<'a> {
    description: String,
    tags: String,
    tags_split: Vec<String>,
    temporary_file_filename: String,
    temporary_file_filepath: String,
    uploader_public_key: String,
    validation_alert: Option<AlertTemplate<'a>>,
}
impl<'a> UploadTemplate<'a> {
    pub async fn new(context: &'a UploadPageContext) -> Result<UploadTemplate<'a>, Box<dyn Error>> {
        let validation_alert = get_validation_alert(&context.params.validation_report);

        let description = context.params.description.clone();
        let temporary_file_filename = context.params.temporary_file_filename.clone();
        let tags = context.params.tags.clone();
        let tags_split = tags.split(",")
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        let uploader_public_key = context.params.uploader_public_key.clone();

        let temporary_file_filepath = if !context.params.temporary_file_filename.is_empty() {
            format!("/assets/images/tmp/{}", &context.params.temporary_file_filename)
        } else {
            String::from("")
        };

        Ok(UploadTemplate {
            description,
            tags,
            tags_split,
            temporary_file_filename,
            temporary_file_filepath,
            uploader_public_key,
            validation_alert,
        })
    }
}

fn get_validation_alert<'a>(report: &Option<Report>) -> Option<AlertTemplate<'a>> {
    match report {
        Some(report) => {
            let mut message_html: String = "".to_owned();

            if report_has_field(report, "server_error") {
                message_html.push_str("<p>A system error occurred. Please notify the site admins if this continues to happen.</p>");
            }
            if report_has_field(report, "file_exist") {
                message_html.push_str("<p>This file has already been uploaded.</p>");
            }
            if report_has_field(report, "temporary_file_filename") {
                message_html.push_str("<p><strong>File:</strong> Please upload a png, gif, or webp file that is less than 12 megabytes large.</p>");
            }
            if report_has_field(report, "image_transfer") {
                message_html.push_str("<p>Image upload failed. Please notify the site admins if this continues to happen.</p>");
            }
            if report_has_field(report, "image_parse") {
                message_html.push_str("<p>This doesn't look like a valid image file that we accept. Please try a different image or re-encode it with another tool as a GIF.</p>");
            }
            if report_has_field(report, "image_not_animated") {
                message_html.push_str("<p>The uploaded image does not appear to be animated. We only accept animations.</p>");
            }
            if report_has_field(report, "image_already_exists") {
                message_html.push_str("<p>This image was already added to the GIF database before. Thanks for your submission!</p>");
            }
            if report_has_field(report, "description") {
                message_html.push_str("<p><strong>Description:</strong> This is required and cannot be longer than 256 characters.</p>");
            }
            if report_has_field(report, "tags") {
                message_html.push_str("<p><strong>Tags:</strong> Please add at least 3 tags.</p>");
            }

            Some(AlertTemplate {
                variant: "danger",
                message_html,
            })
        },
        _ => None,
    }
}
