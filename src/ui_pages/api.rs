use std::error::Error;
use std::marker::PhantomData;
use askama::Template;

use crate::router::routes::api::ApiPageContext;

use crate::util::secrets::{ secrets_config };

#[derive(Template)]
#[template(path = "ui_pages/api.html", blocks = ["page_content"])]
pub struct ApiTemplate<'a> {
    website_host: &'a str,
}
impl<'a> ApiTemplate<'a> {
    pub async fn new(_context: &'a ApiPageContext) -> Result<ApiTemplate<'a>, Box<dyn Error>> {

        let website_host = &secrets_config().website.host;

        Ok(ApiTemplate { website_host })
    }
}

