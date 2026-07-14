use axum::{
    http::{ StatusCode },
    response::{ IntoResponse, Response, Redirect },
};
use askama::Template;
use garde::{
    Validate, Report,
};
use lettre::message::header::ContentType;
use macros::{ RouteParamsContext, render_template };
use std::collections::HashSet;

use crate::database::{ self, ModerationCounterClaim };
use crate::ui_pages::counter_claim::{ CounterClaimTemplate };
use crate::router::{ html_to_response };
use crate::router::context::{ BaseContext, Context, RouteParamContextGenerator };
use crate::router::validation::{ create_simple_report };
use crate::util::smtp::send_email;
use crate::util::secrets::secrets_config;

#[derive(Default, RouteParamsContext)]
pub struct CounterClaimPageParams {
    #[route_param_source(source = "none")]
    pub validation_report: Option<Report>,

    #[route_param_source(source = "path", name = "cid", default = "")]
    pub cid: String,

    #[route_param_source(source = "none")]
    pub counter_claimant_name: String,

    #[route_param_source(source = "none")]
    pub counter_claimant_mailing_address: String,

    #[route_param_source(source = "none")]
    pub counter_claimant_phone: String,

    #[route_param_source(source = "none")]
    pub counter_claimant_email: String,

    #[route_param_source(source = "none")]
    pub counter_claimant_good_faith_attestation: String,

    #[route_param_source(source = "none")]
    pub counter_claimant_service_attestation: String,
}
pub type CounterClaimPageContext = BaseContext<CounterClaimPageParams>;

pub async fn get_counter_claim(
    Context { context }: Context<CounterClaimPageParams>,
) -> Response {
    html_to_response(
        &context,
        |hx_target, context| async move {
            match hx_target.as_str() {
                "main-article" => render_template!(CounterClaimTemplate, &context, page_content),
                _ => render_template!(CounterClaimTemplate, &context),
            }
        }
    ).await
}

#[derive(Default, Debug, RouteParamsContext, Validate)]
pub struct PostCounterClaimPageParams {
    #[route_param_source(source = "path", name = "cid", default = "")]
    #[garde(skip)]
    pub cid: String,

    #[route_param_source(source = "form", name = "counter-claimant-public-key", default = "")]
    #[garde(
        length(max = 64)
    )]
    pub counter_claimant_public_key: String,

    #[route_param_source(source = "form", name = "counter-claimant-name", default = "")]
    #[garde(
        length(min = 1, max = 256),
    )]
    pub counter_claimant_name: String,

    #[route_param_source(source = "form", name = "counter-claimant-mailing-address", default = "")]
    #[garde(
        length(min = 1, max = 512),
    )]
    pub counter_claimant_mailing_address: String,

    #[route_param_source(source = "form", name = "counter-claimant-phone", default = "")]
    #[garde(
        length(min = 1, max = 32),
    )]
    pub counter_claimant_phone: String,

    #[route_param_source(source = "form", name = "counter-claimant-email", default = "")]
    #[garde(
        length(min = 1, max = 320),
        email,
    )]
    pub counter_claimant_email: String,

    #[route_param_source(source = "form", name = "counter-claimant-good-faith-attestation", default = "")]
    #[garde(
        custom(is_selected_checkbox(&self.counter_claimant_good_faith_attestation))
    )]
    pub counter_claimant_good_faith_attestation: String,

    #[route_param_source(source = "form", name = "counter-claimant-service-attestation", default = "")]
    #[garde(
        custom(is_selected_checkbox(&self.counter_claimant_service_attestation))
    )]
    pub counter_claimant_service_attestation: String,
}

#[axum::debug_handler]
pub async fn post_counter_claim(
    Context { context }: Context<PostCounterClaimPageParams>,
) -> Response {

    let mut page_context = context.clone_with_params(CounterClaimPageParams {
        validation_report: None,
        cid: context.params.cid.clone(),
        counter_claimant_name: context.params.counter_claimant_name.clone(),
        counter_claimant_mailing_address: context.params.counter_claimant_mailing_address.clone(),
        counter_claimant_phone: context.params.counter_claimant_phone.clone(),
        counter_claimant_email: context.params.counter_claimant_email.clone(),
        counter_claimant_good_faith_attestation: context.params.counter_claimant_good_faith_attestation.clone(),
        counter_claimant_service_attestation: context.params.counter_claimant_service_attestation.clone(),
    });

    let validation_result = validate_counter_claim_form(&context.params).await;
    if let Err(report) = validation_result {
        page_context.params.validation_report = Some(report);
        return send_counter_claim_page_response(StatusCode::BAD_REQUEST, page_context).await;
    }

    let gif_id_option = match database::get_gif_by_cid(&context.params.cid).await {
        Ok(gif) => Some(gif.id),
        Err(_) => None,
    };
    if gif_id_option.is_none() {
        tracing::warn!("The GIF the user is reporting is not found. CID: {:?}", context.params.cid);
        page_context.params.validation_report = Some(
            create_simple_report("server_error".to_string(), "Missing GIF.".to_string())
        );
        return send_counter_claim_page_response(StatusCode::INTERNAL_SERVER_ERROR, page_context).await;
    }
    let gif_id = gif_id_option.unwrap();

    let reports_option = match database::get_moderation_reports_by_gif_id(gif_id).await {
        Ok(reports) => Some(reports),
        Err(_) => None,
    };
    if reports_option.is_none() {
        tracing::warn!("Failed to query database for reports for GIF CID: {:?}", context.params.cid);
        page_context.params.validation_report = Some(
            create_simple_report("server_error".to_string(), "Report query failed.".to_string())
        );
        return send_counter_claim_page_response(StatusCode::INTERNAL_SERVER_ERROR, page_context).await;
    }

    let reports = reports_option.unwrap();
    if reports.len() == 0 {
        tracing::warn!("No reports found for GIF CID: {:?}", context.params.cid);
        page_context.params.validation_report = Some(
            create_simple_report("server_error".to_string(), "No reports found.".to_string())
        );
        return send_counter_claim_page_response(StatusCode::INTERNAL_SERVER_ERROR, page_context).await;
    }

    let mut reports_iterator = reports.into_iter().peekable();
    let mut reporter_info_email_body = String::new();
    let mut claimant_count: u64 = 1;
    while let Some(report) = reports_iterator.next() {
        reporter_info_email_body.push_str(format!(r#"
            <h2>Claimant #{}</h2>
            <p>Copyright Holder: {}</p>
            <p>Agent Name: {}</p>
            <p>Phone: {}</p>
            <p>Email: {}</p>
            <p>Mailing Address: {}</p>
        "#,
            claimant_count,
            &report.copyright_holder_name,
            &report.reporter_name,
            &report.reporter_phone,
            &report.reporter_email,
            &report.reporter_mailing_address,
        ).as_str());

        let counter_claim = ModerationCounterClaim {
            report_id: report.id,
            counter_claimant_public_key: context.params.counter_claimant_public_key.clone(),
            counter_claimant_ip_address: context.ip_address.clone(),
            counter_claimant_name: context.params.counter_claimant_name.clone(),
            counter_claimant_mailing_address: context.params.counter_claimant_mailing_address.clone(),
            counter_claimant_phone: context.params.counter_claimant_phone.clone(),
            counter_claimant_email: context.params.counter_claimant_email.clone(),
            counter_claimant_attestation: String::from("I have good faith belief that this material is not authorized by the copyright owner, its agent, or the law. Under penalty of perjury, I attest that the information provided is accurate and I am authorized to make the complaint on behalf of the copyright owner."),
            ..ModerationCounterClaim::default()
        };

        let counter_claim_result = database::create_moderation_counter_claim(counter_claim).await;
        if let Err(counter_claim_error) = counter_claim_result {
            tracing::warn!("A database error occurred when trying to create a moderation counter claim. {:?}", counter_claim_error);
            page_context.params.validation_report = Some(
                create_simple_report("server_error".to_string(), "Error creating counter claim.".to_string())
            );
            return send_counter_claim_page_response(StatusCode::INTERNAL_SERVER_ERROR, page_context).await;
        }

        let _ = send_email(
            &context.params.counter_claimant_email,
            secrets_config().contact.dcma_email.as_str(),
            format!("DCMA Counter Notice For IPFS CID {}", &context.params.cid),
            format!(r#"
                <p>A designated agent has sent a DCMA counter notice for the IPFS image with CID {}</p>
                <p>The designated agent's contact information is as follows:</p>
                <p>Name: {}</p>
                <p>Phone: {}</p>
                <p>Email: {}</p>
                <p>Mailing Address: {}</p>
                <p>If you wish to contest this claim, please file legal action within 10 days of this notice.
                Once you have initiated legal action, follow the link below to notify us. If you do not file
                legal action within 10 days, we may restore the contested content on the website.</p>
                <p><a href="https://{}/notify-legal-action/{}/{}/">Follow this link if you have already filed legal action against the designated agent above.</a></p>
            "#,
                &context.params.cid,
                &context.params.counter_claimant_name,
                &context.params.counter_claimant_phone,
                &context.params.counter_claimant_email,
                &context.params.counter_claimant_mailing_address,
                secrets_config().website.hostname,
                &report.reporter_public_key,
                &context.params.cid,
            ),
            ContentType::TEXT_HTML,
        );

        claimant_count += 1;
    }

    let _ = send_email(
        &context.params.counter_claimant_email,
        secrets_config().contact.dcma_email.as_str(),
        String::from("DCMA Counter Notice Received"),
        format!(r#"
            <p>We have received your counter notice for the GIF with IPFS CID {}</p>
            <p>We have sent a notice to the original reporter of the DCMA infringement on your behalf. If they do not respond with notification that they are filing legal action within 10 days, the GIF may be restored.</p>
            <p>See the following information on the original claimant(s):<p>
            {}
        "#, context.params.cid.clone(), reporter_info_email_body),
        ContentType::TEXT_HTML,
    );

    return Redirect::to(
        format!("/counter-claim/{}/?counter-claim-submitted=true", context.params.cid).as_str()
    ).into_response();
}

async fn validate_counter_claim_form(form: &PostCounterClaimPageParams) -> Result<(), Report> {
    if let Err(report) = form.validate() {
        return Err(report);
    }
    Ok(())
}

pub async fn send_counter_claim_page_response(status: StatusCode, context: CounterClaimPageContext) -> Response {
    (
        status,
        html_to_response(
            &context,
            |hx_target, context| async move {
                match hx_target.as_str() {
                    "main-article" => render_template!(CounterClaimTemplate, &context, page_content),
                    _ => render_template!(CounterClaimTemplate, &context),
                }
            }
        ).await
    ).into_response()
}


fn is_selected_checkbox<'a>(value: &'a str) -> impl FnOnce(&str, &()) -> garde::Result + 'a {
    move |_, _| {
        if value == "true" {
            Ok(())
        } else {
            Err(garde::Error::new("Field is required."))
        }
    }
}
