use axum::{
    response::{ Response },
};
use askama::Template;
use macros::{ RouteParamsContext, render_template };

use crate::ui_pages::gif::{ GifTemplate };
use crate::router::{ html_to_response };
use crate::router::context::{ BaseContext, Context, RouteParamContextGenerator };

#[derive(Default, RouteParamsContext)]
pub struct GifPageParams {
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