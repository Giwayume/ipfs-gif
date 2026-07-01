use axum::{
    http::{ StatusCode },
    response::{ IntoResponse, Response, Redirect },
};
use askama::Template;
use garde::{ Validate, Report };
use macros::{ RouteParamsContext, render_template };
use std::collections::HashSet;

use crate::ui_pages::upload::{ UploadTemplate };
use crate::util::image_upload::{ transfer_temporary_image_to_ipfs };
use crate::router::{ html_to_response, get_hx_target };
use crate::router::context::{ BaseContext, Context, RouteParamContextGenerator };
use crate::router::validation::{ create_simple_report };

#[derive(Default, RouteParamsContext)]
pub struct GetUploadPageParams {
    #[route_param_source(source = "none")]
    pub validation_report: Option<Report>,

    #[route_param_source(default = "")]
    pub description: String,

    #[route_param_source(default = "")]
    pub tags: String,

    #[route_param_source(default = "")]
    pub temporary_file_filename: String,
}
pub type UploadPageContext = BaseContext<GetUploadPageParams>;

pub async fn get_upload(
    Context { context }: Context<GetUploadPageParams>,
) -> Response {
    html_to_response(
        &context,
        |hx_target, context| async move {
            match hx_target.as_str() {
                "main-article" => render_template!(UploadTemplate, &context, page_content),
                "upload-tags" => render_template!(UploadTemplate, &context, upload_tags),
                _ => render_template!(UploadTemplate, &context),
            }
        }
    ).await
}

#[derive(Default, Debug, RouteParamsContext, Validate)]
pub struct PostUploadPageParams {
    #[route_param_source(source = "form", name = "description", default = "")]
    #[garde(
        length(min = 1, max = 256),
    )]
    pub description: String,

    #[route_param_source(source = "form", name = "tags", default = "")]
    #[garde(
        length(max = 4096),
        custom(is_minimum_tag_count(&self.tags, &self.new_tag_name)),
    )]
    pub tags: String,

    #[route_param_source(source = "form", name = "new-tag-name", default = "")]
    #[garde(
        length(max = 4096), // Multiple comma-separated tags may be typed, so this is longer
    )]
    pub new_tag_name: String,

    #[route_param_source(source = "form", name = "delete-tag-name", default = "")]
    #[garde(
        length(max = 256),
    )]
    pub delete_tag_name: String,

    #[route_param_source(source = "form", name = "file", default = "")]
    #[garde(skip)]
    pub file_upload: String,

    #[route_param_source(source = "form", name = "temporary-file", default = "")]
    #[garde(
        custom(is_valid_image_upload(&self.temporary_file_filename, &self.file_upload)),
    )]
    pub temporary_file_filename: String,
}

pub async fn post_upload(
    Context { context }: Context<PostUploadPageParams>,
) -> Response {

    let temporary_file_filename = if context.params.file_upload.is_empty() {
        context.params.temporary_file_filename.clone()
    } else {
        context.params.file_upload.clone()
    };

    let mut page_context = context.clone_with_params(GetUploadPageParams {
        validation_report: None,
        description: context.params.description.clone(),
        tags: merge_tags(&context.params.tags, &context.params.new_tag_name, &context.params.delete_tag_name),
        temporary_file_filename: temporary_file_filename.clone(),
    });

    let validation_result = validate_upload_form(&context.params).await;
    if let Err(report) = validation_result {
        page_context.params.validation_report = Some(report);
        return send_upload_page_response(StatusCode::BAD_REQUEST, page_context).await;
    }

    let hx_target = get_hx_target(&context.route_headers);

    // Don't submit the form if just updating the tags.
    if hx_target == "upload-tags" {
        return send_upload_page_response(StatusCode::OK, page_context).await;
    }

    let ipfs_transfer_result = transfer_temporary_image_to_ipfs(&context.params.temporary_file_filename).await;
    if let Err(transfer_error) = ipfs_transfer_result {
        page_context.params.validation_report = Some(
            create_simple_report(String::from("image_transfer"), String::from("Image transfer failed."))
        );
        return send_upload_page_response(StatusCode::INTERNAL_SERVER_ERROR, page_context).await;
    }
    let cid = ipfs_transfer_result.unwrap();
    tracing::info!("Uploaded image with CID {}", cid);

    // TODO - replace with a redirect
    return send_upload_page_response(StatusCode::BAD_REQUEST, page_context).await;
}

fn merge_tags(tags: &str, new_tag_name: &str, delete_tag_name: &str) -> String {
    let allowed = regex::Regex::new(r"[^A-Za-z0-9 ]+").unwrap();
    tags.split(",")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| allowed.replace_all(&s[..s.len().min(256)], "").to_string().to_lowercase())
        .chain(
            new_tag_name.split(",")
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| allowed.replace_all(&s[..s.len().min(256)], "").to_string().to_lowercase())
        )
        .filter(|s| delete_tag_name.len() == 0 || s != delete_tag_name)
        .collect::<HashSet<String>>()
        .into_iter().collect::<Vec<String>>()
        .join(",")
}

async fn validate_upload_form(form: &PostUploadPageParams) -> Result<(), Report> {
    if let Err(report) = form.validate() {
        return Err(report);
    }
    Ok(())
}

pub async fn send_upload_page_response(status: StatusCode, context: UploadPageContext) -> Response {
    (
        status,
        html_to_response(
            &context,
            |hx_target, context| async move {
                match hx_target.as_str() {
                    "main-article" => render_template!(UploadTemplate, &context, page_content),
                    "upload-tags" => render_template!(UploadTemplate, &context, upload_tags),
                    _ => render_template!(UploadTemplate, &context),
                }
            }
        ).await
    ).into_response()
}

fn is_valid_image_upload<'a>(new_filename: &'a str, existing_filename: &'a str) -> impl FnOnce(&str, &()) -> garde::Result + 'a {
    move |_, _| {
        if new_filename.is_empty() && existing_filename.is_empty() {
            Err(garde::Error::new("Missing file."))
        } else {
            Ok(())
        }
    }
}

fn is_minimum_tag_count<'a>(tags: &'a str, new_tag_name: &'a str) -> impl FnOnce(&str, &()) -> garde::Result + 'a {
    move |_, _| {
        let allowed = regex::Regex::new(r"[^A-Za-z0-9 ]+").unwrap();
        let tag_count = tags.split(",")
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| allowed.replace_all(s, "").to_string().to_lowercase())
            .chain(
                new_tag_name.split(",")
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| allowed.replace_all(s, "").to_string().to_lowercase())
            )
            .collect::<HashSet<String>>()
            .len();
        if tag_count < 3 {
            Err(garde::Error::new("At least 3 tags are required."))
        } else {
            Ok(())
        }
    }
}
