use axum::{
    http::{ StatusCode },
    response::{ IntoResponse, Response },
};
use askama::Template;
use garde::{ Validate, Report };
use macros::{ RouteParamsContext, render_template };
use std::collections::HashSet;

use crate::database;
use crate::util::{ crypto };
use crate::ui_pages::gif::{ GifTemplate };
use crate::router::{ html_to_response };
use crate::router::context::{ BaseContext, Context, RouteParamContextGenerator };
use crate::router::validation::{ create_simple_report };

#[derive(Default, RouteParamsContext)]
pub struct GifPageParams {
    #[route_param_source(source = "none")]
    pub validation_report: Option<Report>,

    #[route_param_source(source = "path", name = "cid", default = "")]
    pub cid: String,
}
pub type GifPageContext = BaseContext<GifPageParams>;

pub async fn get_gif(
    Context { context }: Context<GifPageParams>,
) -> Response {
    html_to_response(
        &context,
        |hx_target, context| async move {
            match hx_target.as_str() {
                "main-article" => render_template!(GifTemplate, &context, page_content),
                _ => render_template!(GifTemplate, &context),
            }
        }
    ).await
}

#[derive(Default, Debug, RouteParamsContext, Validate)]
pub struct PostGifPageParams {
    #[route_param_source(source = "path", name = "cid", default = "")]
    #[garde(skip)]
    pub cid: String,

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

    #[route_param_source(source = "form", name = "upload-signed-message", default = "")]
    #[garde(
        length(max = 128),
    )]
    pub upload_signed_message: String,
}

#[axum::debug_handler]
pub async fn post_gif(
    Context { context }: Context<PostGifPageParams>,
) -> Response {

    let mut page_context = context.clone_with_params(GifPageParams {
        validation_report: None,
        cid: context.params.cid.clone(),
    });

    let validation_result = validate_gif_form(&context.params).await;
    if let Err(report) = validation_result {
        page_context.params.validation_report = Some(report);
        return send_gif_page_response(StatusCode::BAD_REQUEST, page_context).await;
    }

    let maybe_gif = match database::get_gif_by_cid(&context.params.cid).await {
        Ok(gif) => Some(gif),
        Err(_) => None,
    };
    if maybe_gif.is_none() {
        page_context.params.validation_report = Some(
            create_simple_report("server_error".to_string(), "Missing GIF.".to_string())
        );
        return send_gif_page_response(StatusCode::BAD_REQUEST, page_context).await;
    }
    let gif = maybe_gif.unwrap();
    
    if gif.uploader_public_key.trim().len() == 0 {
        page_context.params.validation_report = Some(
            create_simple_report(String::from("server_error"), String::from("Can't fetch list of tags."))
        );
        return send_gif_page_response(StatusCode::BAD_REQUEST, page_context).await;
    }

    let messages_to_sign = Vec::from([
        crypto::random_message_to_sign_now_window(&gif.uploader_public_key, 900, 900, 0),
        crypto::random_message_to_sign_now_window(&gif.uploader_public_key, 900, 900, 1)
    ]);
    let mut signature_verification: Option<()> = None;
    for message_to_sign in messages_to_sign {
        if let Ok(_) = crypto::verify_ed25519_signature(&gif.uploader_public_key, &message_to_sign, &context.params.upload_signed_message) {
            signature_verification = Some(());
            break;
        }
    }

    if signature_verification.is_none() {
        page_context.params.validation_report = Some(
            create_simple_report(String::from("upload_signed_message"), String::from("Error validating uploader signature."))
        );
        return send_gif_page_response(StatusCode::BAD_REQUEST, page_context).await;
    }

    if context.params.delete_tag_name.len() > 0 {
        let _ = database::remove_tag_from_gif(gif.id, &context.params.delete_tag_name).await;
    }

    let new_tags = tags_from_input(&context.params.new_tag_name);

    for tag in new_tags {
        let create_tag_result = database::create_tag(&tag).await;
        if let Err(create_tag_error) = create_tag_result {
            tracing::info!("{:?}", create_tag_error);
            continue;
        }
        let tag_id = create_tag_result.unwrap();
        tracing::info!("tg id {}", tag_id);
        let _ = database::add_tag_to_gif(gif.id, tag_id).await;
    }
    
    send_gif_page_response(StatusCode::OK, page_context).await
}

fn tags_from_input(new_tag_name: &str) -> Vec<String> {
    let allowed = regex::Regex::new(r"[^A-Za-z0-9 ]+").unwrap();
    new_tag_name.split(",")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| allowed.replace_all(&s[..s.len().min(256)], "").to_string().to_lowercase())
        .collect::<HashSet<String>>()
        .into_iter().collect::<Vec<String>>()
}

async fn validate_gif_form(form: &PostGifPageParams) -> Result<(), Report> {
    if let Err(report) = form.validate() {
        return Err(report);
    }
    Ok(())
}

pub async fn send_gif_page_response(status: StatusCode, context: GifPageContext) -> Response {
    (
        status,
        html_to_response(
            &context,
            |hx_target, context| async move {
                match hx_target.as_str() {
                    "main-article" => render_template!(GifTemplate, &context, page_content),
                    "upload-tags" => render_template!(GifTemplate, &context, upload_tags),
                    _ => render_template!(GifTemplate, &context),
                }
            }
        ).await
    ).into_response()
}
