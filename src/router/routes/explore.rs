use axum::{
    response::{ Response },
};
use askama::Template;
use macros::{ RouteParamsContext, render_template };

use crate::ui_pages::explore::{ ExploreTemplate };
use crate::router::{ html_to_response };
use crate::router::context::{ BaseContext, Context, RouteParamContextGenerator };

#[derive(Default, RouteParamsContext)]
pub struct ExplorePageParams {
}
pub type ExplorePageContext = BaseContext<ExplorePageParams>;

pub async fn get_explore(
    Context { context }: Context<ExplorePageParams>,
) -> Response {
    html_to_response(
        &context,
        |hx_target, context| async move {
            match hx_target.as_str() {
                "main-article" => render_template!(ExploreTemplate, &context, page_content),
                _ => render_template!(ExploreTemplate, &context),
            }
        }
    ).await
}
