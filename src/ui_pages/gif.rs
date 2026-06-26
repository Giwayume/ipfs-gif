use std::error::Error;
use std::marker::PhantomData;
use askama::Template;

use crate::router::routes::gif::GifPageContext;
use crate::database::{ self, Gif };

#[derive(Template)]
#[template(path = "ui_pages/gif.html", blocks = ["page_content"])]
pub struct GifTemplate<'a> {
    active_page: &'a str,
    gif: Gif,
    tags: Vec<String>,
}
impl<'a> GifTemplate<'a> {
    pub async fn new(context: &'a GifPageContext) -> Result<GifTemplate<'a>, Box<dyn Error>> {
        let active_page: &str = "home";

        let gif = database::get_gif_by_cid(&context.params.cid).await?;

        let tags = Vec::new();

        Ok(GifTemplate { active_page, gif, tags })
    }
}

