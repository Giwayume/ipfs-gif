use axum::{
    response::{ Response },
};
use askama::Template;
use macros::{ RouteParamsContext, render_template };

use crate::ui_pages::home::{ HomeTemplate, HomeContentTemplate };
use crate::router::{ html_to_response };
use crate::router::context::{ BaseContext, Context, RouteParamContextGenerator };

#[derive(Default, RouteParamsContext)]
pub struct HomePageParams {}
pub type HomePageContext = BaseContext<HomePageParams>;

pub async fn get_home(
    Context { context }: Context<HomePageParams>,
) -> Response {
    html_to_response(
        &context,
        |hx_target, context| async move {
            match hx_target.as_str() {
                "main-article" => render_template!(HomeContentTemplate, &context),
                _ => render_template!(HomeTemplate, &context),
            }
        }
    ).await
}
