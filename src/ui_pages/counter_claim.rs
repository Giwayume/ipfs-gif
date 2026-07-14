use std::error::Error;
use askama::Template;
use garde::{ Report };

use crate::database::{ self, Gif };
use crate::router::routes::counter_claim::CounterClaimPageContext;
use crate::router::validation::report_has_field;
use crate::ui_primitives::alert::AlertTemplate;

#[derive(Template)]
#[template(path = "ui_pages/counter_claim.html", blocks = ["page_content"])]
pub struct CounterClaimTemplate<'a> {
    _phantom: std::marker::PhantomData<&'a ()>,
    gif: Gif,
    counter_claimant_name: String,
    counter_claimant_mailing_address: String,
    counter_claimant_phone: String,
    counter_claimant_email: String,
    counter_claimant_good_faith_attestation: String,
    counter_claimant_service_attestation: String,
    counter_claim_submitted_alert: Option<AlertTemplate<'a>>,
    validation_alert: Option<AlertTemplate<'a>>,
}
impl<'a> CounterClaimTemplate<'a> {
    pub async fn new(context: &'a CounterClaimPageContext) -> Result<CounterClaimTemplate<'a>, Box<dyn Error>> {
        let validation_alert = get_validation_alert(&context.params.validation_report);

        let gif = database::get_gif_by_cid(&context.params.cid).await?;

        let counter_claim_submitted_alert = if let Some(_) = context.route_query.get("counter-claim-submitted") {
            Some(AlertTemplate {
                variant: "success",
                message_html: String::from("<p>Thank you, your counter claim has been submitted successfully. Please check your email in a few minutes for next steps.</p>"),
            })
        } else {
            None
        };

        Ok(CounterClaimTemplate {
            _phantom: std::marker::PhantomData,
            gif,
            counter_claimant_name: context.params.counter_claimant_name.clone(),
            counter_claimant_mailing_address: context.params.counter_claimant_mailing_address.clone(),
            counter_claimant_phone: context.params.counter_claimant_phone.clone(),
            counter_claimant_email: context.params.counter_claimant_email.clone(),
            counter_claimant_good_faith_attestation: context.params.counter_claimant_good_faith_attestation.clone(),
            counter_claimant_service_attestation: context.params.counter_claimant_service_attestation.clone(),
            counter_claim_submitted_alert,
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
            if report_has_field(report, "counter_claimant_public_key") {
                message_html.push_str("<p>It appears your client generated an identity key that is too long. Try a different browser or clear the site data and try again.</p>");
            }
            if report_has_field(report, "counter_claimant_name") {
                message_html.push_str("<p><strong>Your Full Legal Name:</strong> This field is required. Please keep it under 256 characters long.</p>");
            }
            if report_has_field(report, "counter_claimant_mailing_address") {
                message_html.push_str("<p><strong>Your Mailing Address:</strong> This field is required. Please keep it under 512 characters long.</p>");
            }
            if report_has_field(report, "counter_claimant_phone") {
                message_html.push_str("<p><strong>Your Phone Number:</strong> This field is required. Please keep it under 32 characters long.</p>");
            }
            if report_has_field(report, "counter_claimant_email") {
                message_html.push_str("<p><strong>Your Email:</strong> This field is required. Please type a valid email address and keep it under 320 characters long.</p>");
            }
            if report_has_field(report, "counter_claimant_good_faith_attestation") || report_has_field(report, "counter_claimant_service_attestation") {
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