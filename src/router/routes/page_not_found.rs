use axum::{
    http::StatusCode,
    response::{ IntoResponse, Response }
};
use askama::Template;

use macros::{ RouteParamsContext, render_template };

use crate::router::context::{ BaseContext, Context, RouteParamContextGenerator };
use crate::ui_pages::page_not_found::{ PageNotFoundTemplate };
use crate::router::{ html_to_response };

#[derive(Default, RouteParamsContext)]
pub struct PageNotFoundParams {}
pub type PageNotFoundContext = BaseContext<PageNotFoundParams>;

pub async fn get_page_not_found(
    Context { context }: Context<PageNotFoundParams>,
) -> Response {
    (
        StatusCode::NOT_FOUND,
        html_to_response(
            &context,
            |hx_target, context| async move {
                match hx_target.as_str() {
                    "main-article" => render_template!(PageNotFoundTemplate, &context, page_content),
                    _ => render_template!(PageNotFoundTemplate, &context),
                }
            }
        ).await
    ).into_response()
}
