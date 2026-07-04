use std::error::Error;
use askama::Template;
use garde::{ Report };

use crate::router::routes::search::SearchPageContext;
use crate::router::{ get_hx_target };
use crate::database::{ self, Gif };

#[derive(Template)]
#[template(path = "ui_pages/search.html", blocks = ["page_content"])]
pub struct SearchTemplate<'a> {
    query: &'a str,
    gifs: Vec<Gif>,
    needs_title_update: bool,
}
impl<'a> SearchTemplate<'a> {
    pub async fn new(context: &'a SearchPageContext) -> Result<SearchTemplate<'a>, Box<dyn Error>> {
        let needs_title_update = if get_hx_target(&context.route_headers).len() > 0 { true } else { false };

        let gifs = database::search_by_tags(&context.params.query, 60).await?;

        Ok(SearchTemplate {
            query: &context.params.query,
            gifs,
            needs_title_update,
        })
    }
}
