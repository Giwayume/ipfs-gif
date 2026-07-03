use std::error::Error;
use askama::Template;
use garde::{ Report };

use crate::router::routes::gif::GifPageContext;
use crate::router::validation::report_has_field;
use crate::database::{ self, Gif };
use crate::ui_primitives::alert::AlertTemplate;
use crate::util::{ format, crypto };

#[derive(Template)]
#[template(path = "ui_pages/gif.html", blocks = ["page_content", "upload_tags"])]
pub struct GifTemplate<'a> {
    already_uploaded_alert: Option<AlertTemplate<'a>>,
    gif: Gif,
    tags: Vec<(String, String)>,
    update_signing_message: String,
    validation_alert: Option<AlertTemplate<'a>>,
}
impl<'a> GifTemplate<'a> {
    pub async fn new(context: &'a GifPageContext) -> Result<GifTemplate<'a>, Box<dyn Error>> {
        let validation_alert = get_validation_alert(&context.params.validation_report);

        let already_uploaded_alert = if let Some(_) = context.route_query.get("already-uploaded") {
            Some(AlertTemplate {
                variant: "info",
                message_html: String::from("<p>Someone else already uploaded this GIF! We've taken you to it.</p>"),
            })
        } else {
            None
        };

        let gif = database::get_gif_by_cid(&context.params.cid).await?;

        let update_signing_message = if gif.uploader_public_key.len() > 0 {
            crypto::random_message_to_sign_now_window(&gif.uploader_public_key, 900, 900, 0)
        } else {
            String::from("")
        };

        let tags = database::get_tags_by_gif_id(gif.id).await?
            .into_iter()
            .map(|t| (format::to_kebab_case(&t.name), t.name))
            .collect::<Vec<(String, String)>>();

        Ok(GifTemplate { already_uploaded_alert, gif, tags, update_signing_message, validation_alert })
    }
}

fn get_validation_alert<'a>(report: &Option<Report>) -> Option<AlertTemplate<'a>> {
    match report {
        Some(report) => {
            let mut message_html: String = "".to_owned();

            if report_has_field(report, "server_error") {
                message_html.push_str("<p>A system error occurred. Please notify the site admins if this continues to happen.</p>");
            }
            if report_has_field(report, "new_tag_name") {
                message_html.push_str("<p>The tag name you entered is too long.</p>");
            }
            if report_has_field(report, "upload_signed_message") {
                message_html.push_str("<p>We can't verify that you are the original uploader. This can happen if you sit on the page too long before making updates. Try reloading the page.</p>");
            }

            Some(AlertTemplate {
                variant: "danger",
                message_html,
            })
        },
        _ => None,
    }
}
