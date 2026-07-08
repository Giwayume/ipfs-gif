use std::error::Error;
use askama::Template;
use garde::{ Report };

use crate::database::{ self, Gif, GifModerationStatus };
use crate::router::routes::report::ReportPageContext;
use crate::router::validation::report_has_field;
use crate::ui_primitives::alert::AlertTemplate;

#[derive(Template)]
#[template(path = "ui_pages/report.html", blocks = ["page_content"])]
pub struct ReportTemplate<'a> {
    _phantom: std::marker::PhantomData<&'a ()>,
    gif: Gif,
    is_gif_visible: bool,
    reason: String,
    copyright_holder_name: String,
    reporter_name: String,
    reporter_mailing_address: String,
    reporter_phone: String,
    reporter_email: String,
    reporter_good_faith_attestation: String,
    reporter_accuracy_attestation: String,
    report_submitted_alert: Option<AlertTemplate<'a>>,
    validation_alert: Option<AlertTemplate<'a>>,
}
impl<'a> ReportTemplate<'a> {
    pub async fn new(context: &'a ReportPageContext) -> Result<ReportTemplate<'a>, Box<dyn Error>> {
        let validation_alert = get_validation_alert(&context.params.validation_report);

        let gif = database::get_gif_by_cid(&context.params.cid).await?;

        let report_submitted_alert = if let Some(_) = context.route_query.get("report-submitted") {
            Some(AlertTemplate {
                variant: "success",
                message_html: String::from("<p>Thank you, your report has been submitted successfully.</p>"),
            })
        } else {
            None
        };

        let is_gif_visible = match gif.moderation_status {
            GifModerationStatus::ManuallyReviewed => true,
            GifModerationStatus::None => true,
            _ => false,
        };

        let reason = context.params.reason.clone();

        Ok(ReportTemplate {
            _phantom: std::marker::PhantomData,
            gif,
            is_gif_visible,
            reason,
            copyright_holder_name: context.params.copyright_holder_name.clone(),
            reporter_name: context.params.reporter_name.clone(),
            reporter_mailing_address: context.params.reporter_mailing_address.clone(),
            reporter_phone: context.params.reporter_phone.clone(),
            reporter_email: context.params.reporter_email.clone(),
            reporter_good_faith_attestation: context.params.reporter_good_faith_attestation.clone(),
            reporter_accuracy_attestation: context.params.reporter_accuracy_attestation.clone(),
            report_submitted_alert,
            validation_alert,
        })
    }
}

fn get_validation_alert<'a>(report: &Option<Report>) -> Option<AlertTemplate<'a>> {
    match report {
        Some(report) => {
            let mut message_html: String = "".to_owned();

            if report_has_field(report, "server_error") || report_has_field(report, "action") {
                message_html.push_str("<p>A system error occurred. Please notify the site admins if this continues to happen.</p>");
            }
            if report_has_field(report, "reason") {
                message_html.push_str("<p>Invalid report reason was selected.</p>");
            }
            if report_has_field(report, "reporter_public_key") {
                message_html.push_str("<p>It appears your client generated an identity key that is too long. Try a different browser or clear the site data and try again.</p>");
            }
            if report_has_field(report, "copyright_holder_name") {
                message_html.push_str("<p><strong>Copyright Holder Name:</strong> This field is required. Please keep it under 256 characters long.</p>");
            }
            if report_has_field(report, "reporter_name") {
                message_html.push_str("<p><strong>Your Full Legal Name:</strong> This field is required. Please keep it under 256 characters long.</p>");
            }
            if report_has_field(report, "reporter_mailing_address") {
                message_html.push_str("<p><strong>Your Mailing Address:</strong> This field is required. Please keep it under 512 characters long.</p>");
            }
            if report_has_field(report, "reporter_phone") {
                message_html.push_str("<p><strong>Your Phone Number:</strong> This field is required. Please keep it under 32 characters long.</p>");
            }
            if report_has_field(report, "reporter_email") {
                message_html.push_str("<p><strong>Your Email:</strong> This field is required. Please type a valid email address and keep it under 320 characters long.</p>");
            }
            if report_has_field(report, "reporter_good_faith_attestation") || report_has_field(report, "reporter_accuracy_attestation") {
                message_html.push_str("<p>Please check both checkboxes at the bottom of the form to indicate that you attest to both the accuracy and authorization of this report.</p>");
            }

            Some(AlertTemplate {
                variant: "danger",
                message_html,
            })
        },
        _ => None,
    }
}