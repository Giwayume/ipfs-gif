use axum::{
    response::{ Response },
};
use askama::Template;
use macros::{ RouteParamsContext, render_template };

use crate::ui_pages::moderation_queue::{ ModerationQueueTemplate };
use crate::router::{ html_to_response };
use crate::router::context::{ BaseContext, Context, RouteParamContextGenerator };

#[derive(Default, RouteParamsContext)]
pub struct ModerationQueuePageParams {
}
pub type ModerationQueuePageContext = BaseContext<ModerationQueuePageParams>;

pub async fn get_moderation_queue(
    Context { context }: Context<ModerationQueuePageParams>,
) -> Response {
    html_to_response(
        &context,
        |hx_target, context| async move {
            match hx_target.as_str() {
                "main-article" => render_template!(ModerationQueueTemplate, &context, page_content),
                _ => render_template!(ModerationQueueTemplate, &context),
            }
        }
    ).await
}
