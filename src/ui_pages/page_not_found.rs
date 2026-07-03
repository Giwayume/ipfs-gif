use std::error::Error;
use askama::Template;

use crate::router::routes::page_not_found::{ PageNotFoundContext };

#[derive(Template)]
#[template(path = "ui_pages/page_not_found.html", blocks = ["page_content"])]
pub struct PageNotFoundTemplate<'a> {
    _phantom: std::marker::PhantomData<&'a ()>,
}
impl<'a> PageNotFoundTemplate<'a> {
    pub async fn new(_context: &'a PageNotFoundContext) -> Result<PageNotFoundTemplate<'a>, Box<dyn Error>> {
        Ok(PageNotFoundTemplate {
            _phantom: std::marker::PhantomData,
        })
    }
}