use axum::{
    response::{ Response },
};
use askama::Template;
use macros::{ RouteParamsContext, render_template };

use crate::ui_pages::search::{ SearchTemplate };
use crate::router::{ html_to_response };
use crate::router::context::{ BaseContext, Context, RouteParamContextGenerator };

#[derive(Default, RouteParamsContext)]
pub struct SearchPageParams {
    #[route_param_source(source = "query", name = "q", default = "")]
    pub query: String,
}
pub type SearchPageContext = BaseContext<SearchPageParams>;

pub async fn get_search(
    Context { context }: Context<SearchPageParams>,
) -> Response {
    html_to_response(
        &context,
        |hx_target, context| async move {
            match hx_target.as_str() {
                "main-article" => render_template!(SearchTemplate, &context, page_content),
                _ => render_template!(SearchTemplate, &context),
            }
        }
    ).await
}
