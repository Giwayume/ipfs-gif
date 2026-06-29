use std::error::Error;
use std::marker::PhantomData;
use askama::Template;

use crate::router::routes::upload::UploadPageContext;

#[derive(Template)]
#[template(path = "ui_pages/upload.html", blocks = ["page_content"])]
pub struct UploadTemplate<'a> {
    active_page: &'a str,
}
impl<'a> UploadTemplate<'a> {
    pub async fn new(context: &'a UploadPageContext) -> Result<UploadTemplate<'a>, Box<dyn Error>> {
        let active_page: &str = "upload";

        Ok(UploadTemplate { active_page })
    }
}
