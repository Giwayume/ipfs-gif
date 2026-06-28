use axum::{
    response::{ Response },
};
use askama::Template;
use macros::{ RouteParamsContext, render_template };

use crate::ui_pages::api::{ ApiTemplate };
use crate::router::{ html_to_response };
use crate::router::context::{ BaseContext, Context, RouteParamContextGenerator };

#[derive(Default, RouteParamsContext)]
pub struct ApiPageParams {
}
pub type ApiPageContext = BaseContext<ApiPageParams>;

pub async fn get_api(
    Context { context }: Context<ApiPageParams>,
) -> Response {
    html_to_response(
        &context,
        |hx_target, context| async move {
            match hx_target.as_str() {
                "main-article" => render_template!(ApiTemplate, &context, page_content),
                _ => render_template!(ApiTemplate, &context),
            }
        }
    ).await
}