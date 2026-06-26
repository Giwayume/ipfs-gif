use std::future::Future;
use axum::{
    http::{ HeaderMap, StatusCode },
    http::header::HeaderValue,
    response::{ Html, IntoResponse, Response, Redirect },
};

use crate::util::error::RenderingError;

pub mod context;
pub mod routes;
pub use routes::initialize;

use context::{ RouteContext };

fn get_hx_target<'a>(headers: &'a HeaderMap) -> &'a str {
    if let Some(hx_target) = headers.get("HX-Target") {
        if let Ok(value) = hx_target.to_str() {
            return value;
        }
    }
    ""
}

async fn html_to_response<'a, Ctx, F, Fut>(context: &'a Ctx, f: F) -> Response
where
    &'a Ctx: RouteContext,
    F: FnOnce(String, &'a Ctx) -> Fut,
    Fut: Future<Output = Result<String, RenderingError>>,
{
    let hx_target = String::from(get_hx_target(context.route_headers()));
    let has_hx_target = !hx_target.is_empty();
    let mut status_code = StatusCode::OK;

    let html: String = match f(hx_target, &context).await {
        Ok(html) => html,
        Err(error) => {
            tracing::info!("Template threw an error during rendering. Sending a 404. {:?}", error);
            status_code = StatusCode::NOT_FOUND;
            String::from("")
        },
    };

    if status_code == StatusCode::NOT_FOUND {
        return Redirect::temporary("/404/").into_response();
    }

    let mut response = (
        status_code,
        Html(html)
    ).into_response();

    let headers = response.headers_mut();
    if has_hx_target {
        headers.insert(
            "Cache-Control",
            HeaderValue::from_static("no-store, max-age=0"),
        );
    }

    response
}
