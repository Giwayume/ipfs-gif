use std::error::Error;
use askama::Template;

use crate::database::{ self, Gif };
use crate::router::routes::report::ReportPageContext;

#[derive(Template)]
#[template(path = "ui_pages/report.html", blocks = ["page_content"])]
pub struct ReportTemplate<'a> {
    _phantom: std::marker::PhantomData<&'a ()>,
    gif: Gif,
}
impl<'a> ReportTemplate<'a> {
    pub async fn new(context: &'a ReportPageContext) -> Result<ReportTemplate<'a>, Box<dyn Error>> {

        let gif = database::get_gif_by_cid(&context.params.cid).await?;

        Ok(ReportTemplate {
            _phantom: std::marker::PhantomData,
            gif,
        })
    }
}

