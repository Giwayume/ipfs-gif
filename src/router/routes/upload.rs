use axum::{
    response::{ Response },
};
use askama::Template;
use macros::{ RouteParamsContext, render_template };

use crate::ui_pages::upload::{ UploadTemplate };
use crate::router::{ html_to_response };
use crate::router::context::{ BaseContext, Context, RouteParamContextGenerator };

#[derive(Default, RouteParamsContext)]
pub struct UploadPageParams {
}
pub type UploadPageContext = BaseContext<UploadPageParams>;

pub async fn get_upload(
    Context { context }: Context<UploadPageParams>,
) -> Response {
    html_to_response(
        &context,
        |hx_target, context| async move {
            match hx_target.as_str() {
                "main-article" => render_template!(UploadTemplate, &context, page_content),
                _ => render_template!(UploadTemplate, &context),
            }
        }
    ).await
}
