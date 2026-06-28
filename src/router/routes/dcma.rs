use axum::{
    response::{ Response },
};
use askama::Template;
use macros::{ RouteParamsContext, render_template };

use crate::ui_pages::dcma::{ DcmaTemplate };
use crate::router::{ html_to_response };
use crate::router::context::{ BaseContext, Context, RouteParamContextGenerator };

#[derive(Default, RouteParamsContext)]
pub struct DcmaPageParams {
}
pub type DcmaPageContext = BaseContext<DcmaPageParams>;

pub async fn get_dcma(
    Context { context }: Context<DcmaPageParams>,
) -> Response {
    html_to_response(
        &context,
        |hx_target, context| async move {
            match hx_target.as_str() {
                "main-article" => render_template!(DcmaTemplate, &context, page_content),
                _ => render_template!(DcmaTemplate, &context),
            }
        }
    ).await
}