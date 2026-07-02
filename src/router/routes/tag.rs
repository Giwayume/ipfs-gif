use axum::{
    response::{ Response },
};
use askama::Template;
use macros::{ RouteParamsContext, render_template };

use crate::ui_pages::tag::{ TagTemplate };
use crate::router::{ html_to_response };
use crate::router::context::{ BaseContext, Context, RouteParamContextGenerator };

#[derive(Default, RouteParamsContext)]
pub struct TagPageParams {
    #[route_param_source(source = "path", name = "tag_hash", default = "")]
    pub tag_hash: String,
}
pub type TagPageContext = BaseContext<TagPageParams>;

pub async fn get_tag(
    Context { context }: Context<TagPageParams>,
) -> Response {
    html_to_response(
        &context,
        |hx_target, context| async move {
            match hx_target.as_str() {
                "main-article" => render_template!(TagTemplate, &context, page_content),
                _ => render_template!(TagTemplate, &context),
            }
        }
    ).await
}