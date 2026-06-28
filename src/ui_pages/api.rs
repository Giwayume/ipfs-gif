use std::error::Error;
use std::marker::PhantomData;
use askama::Template;

use crate::router::routes::api::ApiPageContext;

use crate::util::secrets::{ secrets_config };

#[derive(Template)]
#[template(path = "ui_pages/api.html", blocks = ["page_content"])]
pub struct ApiTemplate<'a> {
    active_page: &'a str,
    website_host: &'a str,
}
impl<'a> ApiTemplate<'a> {
    pub async fn new(context: &'a ApiPageContext) -> Result<ApiTemplate<'a>, Box<dyn Error>> {
        let active_page: &str = "home";

        let website_host = &secrets_config().website.host;

        Ok(ApiTemplate { active_page, website_host })
    }
}

