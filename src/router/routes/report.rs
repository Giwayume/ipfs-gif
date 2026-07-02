use axum::{
    response::{ Response },
};
use askama::Template;
use macros::{ RouteParamsContext, render_template };

use crate::ui_pages::report::{ ReportTemplate };
use crate::router::{ html_to_response };
use crate::router::context::{ BaseContext, Context, RouteParamContextGenerator };

#[derive(Default, RouteParamsContext)]
pub struct ReportPageParams {
    #[route_param_source(source = "path", name = "cid", default = "")]
    pub cid: String,
}
pub type ReportPageContext = BaseContext<ReportPageParams>;

pub async fn get_report(
    Context { context }: Context<ReportPageParams>,
) -> Response {
    html_to_response(
        &context,
        |hx_target, context| async move {
            match hx_target.as_str() {
                "main-article" => render_template!(ReportTemplate, &context, page_content),
                _ => render_template!(ReportTemplate, &context),
            }
        }
    ).await
}