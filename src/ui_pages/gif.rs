use std::error::Error;
use askama::Template;
use garde::{ Report };

use crate::router::routes::gif::GifPageContext;
use crate::router::validation::report_has_field;
use crate::database::{ self, Gif, QuarantineScanResult };
use crate::ui_primitives::alert::AlertTemplate;
use crate::util::{ format, crypto };

#[derive(Template)]
#[template(path = "ui_pages/gif.html", blocks = ["page_content", "upload_tags"])]
pub struct GifTemplate<'a> {
    already_uploaded_alert: Option<AlertTemplate<'a>>,
    cid: String,
    gif: Gif,
    gif_img_src: String,
    quarantined_alert: Option<AlertTemplate<'a>>,
    quarantined_alert_noscript: Option<AlertTemplate<'a>>,
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

        let gif = if let Some(gif) = &context.params.gif {
            gif.clone()
        } else {
            database::get_gif_by_cid(&context.params.cid).await?
        };
        let cid = String::from(gif.cid.as_deref().unwrap_or_else(|| ""));

        let gif_img_src = if gif.cid.is_none() {
            format!("/assets/images/quarantine/{}",
                match gif.quarantine_id
                    .replace("qt-", "")
                    .rsplit_once("-") {
                    Some((left, right)) => format!("{left}.{right}"),
                    _ => gif.quarantine_id.clone(),
                }
            )
        } else {
            format!("https://ipfs.io/ipfs/{}?filename={}", &cid, gif.filename)
        };

        let quarantine_error_message = get_quarantine_scan_message(&gif.quarantine_scan_result);
        let quarantined_alert = if gif.cid.is_none() {
            Some(AlertTemplate {
                variant: if quarantine_error_message.is_some() { "danger" } else { "info" },
                message_html: if let Some(message_html) = quarantine_error_message.clone() {
                    message_html
                } else {
                    String::from("<p>This GIF is being scanned, this can take a minute. This page will refresh when it's ready to share!</p>")
                },
            })
        } else {
            None
        };
        let quarantined_alert_noscript = if gif.cid.is_none() {
            Some(AlertTemplate {
                variant: if quarantine_error_message.is_some() { "danger" } else { "info" },
                message_html: if let Some(message_html) = quarantine_error_message {
                    message_html
                } else {
                    String::from("<p>This GIF is being scanned, this can take a minute. Since you do not have Javascript enabled, refresh this page in a minute. This message will disappear when the GIF is ready to share!</p>")
                },
            })
        } else {
            None
        };

        let update_signing_message = if gif.uploader_public_key.len() > 0 {
            crypto::random_message_to_sign_now_window(&gif.uploader_public_key, 900, 900, 0)
        } else {
            String::from("")
        };

        let tags = database::get_tags_by_gif_id(gif.id).await?
            .into_iter()
            .map(|t| (format::to_kebab_case(&t.name), t.name))
            .collect::<Vec<(String, String)>>();

        Ok(GifTemplate {
            already_uploaded_alert,
            cid,
            gif,
            gif_img_src,
            quarantined_alert,
            quarantined_alert_noscript,
            tags,
            update_signing_message,
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

fn get_quarantine_scan_message<'a>(quarantine_scan_result: &QuarantineScanResult) -> Option<String> {
    match quarantine_scan_result {
        QuarantineScanResult::MissingImage => 
            Some(String::from("<p>A system error occurred when uploading this image. Please try again later.</p>")),
        QuarantineScanResult::ImageParseFailed => 
            Some(String::from("<p>We were unable to decode this image. You can try using a different program to export it and upload again.</p>")),
        QuarantineScanResult::ScanFailed => 
            Some(String::from("<p>We appreciate your submission, but it cannot be accepted at this time.</p>")),
        QuarantineScanResult::IpfsTransferFailed => 
            Some(String::from("<p>A system error occurred when uploading this image. Please try again later.</p>")),
        QuarantineScanResult::IpfsDuplicate => 
            Some(String::from("<p>Someone else has already uploaded this image before. Thanks for submitting!</p>")),
        QuarantineScanResult::None => 
            None,
    }
}