use axum::{
    http::{ StatusCode },
    response::{ IntoResponse, Response, Redirect },
};
use askama::Template;
use garde::{
    Validate, Report,
    rules::email::parse_email,
};
use macros::{ RouteParamsContext, render_template };
use std::collections::HashSet;

use crate::database::{ self, ModerationReport, ModerationReportType };
use crate::ui_pages::report::{ ReportTemplate };
use crate::router::{ html_to_response };
use crate::router::context::{ BaseContext, Context, RouteParamContextGenerator };
use crate::router::validation::{ create_simple_report };

#[derive(Default, RouteParamsContext)]
pub struct ReportPageParams {
    #[route_param_source(source = "none")]
    pub validation_report: Option<Report>,

    #[route_param_source(source = "path", name = "cid", default = "")]
    pub cid: String,

    #[route_param_source(source = "none")]
    pub reason: String,

    #[route_param_source(source = "none")]
    pub copyright_holder_name: String,

    #[route_param_source(source = "none")]
    pub reporter_name: String,

    #[route_param_source(source = "none")]
    pub reporter_mailing_address: String,

    #[route_param_source(source = "none")]
    pub reporter_phone: String,

    #[route_param_source(source = "none")]
    pub reporter_email: String,

    #[route_param_source(source = "none")]
    pub reporter_good_faith_attestation: String,

    #[route_param_source(source = "none")]
    pub reporter_accuracy_attestation: String,
}
pub type ReportPageContext = BaseContext<ReportPageParams>;

pub async fn get_report(
    Context { context }: Context<ReportPageParams>,
) -> Response {
    html_to_response(
        &context,
        |hx_target, context| async move {
            match hx_target.as_str() {
                "main-article" => render_template!(ReportTemplate, &context, page_content),
                _ => render_template!(ReportTemplate, &context),
            }
        }
    ).await
}

#[derive(Default, Debug, RouteParamsContext, Validate)]
pub struct PostReportPageParams {
    #[route_param_source(source = "path", name = "cid", default = "")]
    #[garde(skip)]
    pub cid: String,

    #[route_param_source(source = "form", name = "action", default = "")]
    #[garde(
        length(max = 64)
    )]
    pub action: String,

    #[route_param_source(source = "form", name = "reason", default = "")]
    #[garde(
        custom(is_valid_report_reason(&self.reason))
    )]
    pub reason: String,

    #[route_param_source(source = "form", name = "copyright-holder-name", default = "")]
    #[garde(
        length(max = 256),
        custom(is_required_reporter_info(&self.reason, &self.action, &self.copyright_holder_name))
    )]
    pub copyright_holder_name: String,

    #[route_param_source(source = "form", name = "reporter-public-key", default = "")]
    #[garde(
        length(max = 64)
    )]
    pub reporter_public_key: String,

    #[route_param_source(source = "form", name = "reporter-name", default = "")]
    #[garde(
        length(max = 256),
        custom(is_required_reporter_info(&self.reason, &self.action, &self.reporter_name))
    )]
    pub reporter_name: String,

    #[route_param_source(source = "form", name = "reporter-mailing-address", default = "")]
    #[garde(
        length(max = 512),
        custom(is_required_reporter_info(&self.reason, &self.action, &self.reporter_mailing_address))
    )]
    pub reporter_mailing_address: String,

    #[route_param_source(source = "form", name = "reporter-phone", default = "")]
    #[garde(
        length(max = 32),
        custom(is_required_reporter_info(&self.reason, &self.action, &self.reporter_phone))
    )]
    pub reporter_phone: String,

    #[route_param_source(source = "form", name = "reporter-email", default = "")]
    #[garde(
        length(max = 320),
        custom(is_required_reporter_email(&self.reason, &self.action, &self.reporter_email))
    )]
    pub reporter_email: String,

    #[route_param_source(source = "form", name = "reporter-good-faith-attestation", default = "")]
    #[garde(
        custom(is_required_reporter_attestation(&self.reason, &self.action, &self.reporter_good_faith_attestation))
    )]
    pub reporter_good_faith_attestation: String,

    #[route_param_source(source = "form", name = "reporter-accuracy-attestation", default = "")]
    #[garde(
        custom(is_required_reporter_attestation(&self.reason, &self.action, &self.reporter_accuracy_attestation))
    )]
    pub reporter_accuracy_attestation: String,
}

#[axum::debug_handler]
pub async fn post_report(
    Context { context }: Context<PostReportPageParams>,
) -> Response {

    let mut page_context = context.clone_with_params(ReportPageParams {
        validation_report: None,
        cid: context.params.cid.clone(),
        reason: context.params.reason.clone(),
        copyright_holder_name: context.params.copyright_holder_name.clone(),
        reporter_name: context.params.reporter_name.clone(),
        reporter_mailing_address: context.params.reporter_mailing_address.clone(),
        reporter_phone: context.params.reporter_phone.clone(),
        reporter_email: context.params.reporter_email.clone(),
        reporter_good_faith_attestation: context.params.reporter_good_faith_attestation.clone(),
        reporter_accuracy_attestation: context.params.reporter_accuracy_attestation.clone(),
    });

    let validation_result = validate_report_form(&context.params).await;
    if let Err(report) = validation_result {
        page_context.params.validation_report = Some(report);
        return send_report_page_response(StatusCode::BAD_REQUEST, page_context).await;
    }

    if context.params.action == "select-reason" && context.params.reason == "dcma" {
        return send_report_page_response(StatusCode::OK, page_context).await;
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
        return send_report_page_response(StatusCode::INTERNAL_SERVER_ERROR, page_context).await;
    }
    let gif_id = gif_id_option.unwrap();

    let report_type = match context.params.reason.as_str() {
        "dcma" => ModerationReportType::Dcma,
        "doxxing" => ModerationReportType::Doxxing,
        "gore" => ModerationReportType::Gore,
        "illegal" => ModerationReportType::Illegal,
        "sexual" => ModerationReportType::Sexual,
        _ => {
            tracing::warn!("User tried to submit a report with a non-existent reason. {:?}", context.params.reason);
            page_context.params.validation_report = Some(
                create_simple_report("server_error".to_string(), "Invalid report reason.".to_string())
            );
            return send_report_page_response(StatusCode::INTERNAL_SERVER_ERROR, page_context).await;
        }
    };

    let report = ModerationReport {
        gif_id,
        report_type,
        copyright_holder_name: context.params.copyright_holder_name,
        reporter_public_key: context.params.reporter_public_key,
        reporter_ip_address: context.ip_address.clone(),
        reporter_name: context.params.reporter_name,
        reporter_mailing_address: context.params.reporter_mailing_address,
        reporter_phone: context.params.reporter_phone,
        reporter_email: context.params.reporter_email,
        reporter_attestation: if context.params.reason == "dcma" {
            String::from("I have good faith belief that this material is not authorized by the copyright owner, its agent, or the law. Under penalty of perjury, I attest that the information provided is accurate and I am authorized to make the complaint on behalf of the copyright owner.")
        } else {
            String::from("")
        },
        ..ModerationReport::default()
    };

    let report_result = database::create_moderation_report(report).await;
    if let Err(report_error) = report_result {
        tracing::warn!("A database error occurred when trying to create a moderation report. {:?}", report_error);
        page_context.params.validation_report = Some(
            create_simple_report("server_error".to_string(), "Error creating report.".to_string())
        );
        return send_report_page_response(StatusCode::INTERNAL_SERVER_ERROR, page_context).await;
    }

    return Redirect::to(
        format!("/report/{}/?report-submitted={}", context.params.cid, context.params.reason).as_str()
    ).into_response();
}

async fn validate_report_form(form: &PostReportPageParams) -> Result<(), Report> {
    if let Err(report) = form.validate() {
        return Err(report);
    }
    Ok(())
}

pub async fn send_report_page_response(status: StatusCode, context: ReportPageContext) -> Response {
    (
        status,
        html_to_response(
            &context,
            |hx_target, context| async move {
                match hx_target.as_str() {
                    "main-article" => render_template!(ReportTemplate, &context, page_content),
                    _ => render_template!(ReportTemplate, &context),
                }
            }
        ).await
    ).into_response()
}

fn is_valid_report_reason<'a>(reason: &'a str) -> impl FnOnce(&str, &()) -> garde::Result + 'a {
    move |_, _| {
        let set: HashSet<&str> = ["illegal", "doxxing", "gore", "sexual", "dcma"].into_iter().collect();
        if set.contains(reason) {
            Ok(())
        } else {
            Err(garde::Error::new("Invalid report type."))
        }
    }
}

fn is_required_reporter_info<'a>(reason: &'a str, action: &'a str, value: &'a str) -> impl FnOnce(&str, &()) -> garde::Result + 'a {
    move |_, _| {
        if reason == "dcma" && value.trim().len() == 0 && action != "select-reason" {
            Err(garde::Error::new("Reporter info field is required."))
        } else {
            Ok(())
        }
    }
}

fn is_required_reporter_email<'a>(reason: &'a str, action: &'a str, value: &'a str) -> impl FnOnce(&str, &()) -> garde::Result + 'a {
    move |_, _| {
        if action == "select-reason" {
            Ok(())
        } else if reason == "dcma" && value.trim().len() == 0 {
            Err(garde::Error::new("Reporter email field is required."))
        } else {
            match parse_email(value) {
                Ok(v) => Ok(v),
                Err(_) => Err(garde::Error::new("Reporter email validation failed."))
            }
        }
    }
}

fn is_required_reporter_attestation<'a>(reason: &'a str, action: &'a str, value: &'a str) -> impl FnOnce(&str, &()) -> garde::Result + 'a {
    move |_, _| {
        if reason == "dcma" && value != "true" && action != "select-reason" {
            Err(garde::Error::new("Reporter attestation field is required."))
        } else {
            Ok(())
        }
    }
}
