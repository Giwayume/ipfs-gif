use std::error::Error;
use askama::Template;
use garde::{ Report };

use crate::router::routes::trending::TrendingPageContext;
use crate::router::{ get_hx_target };
use crate::database::{ self, Gif };

#[derive(Template)]
#[template(path = "ui_pages/trending.html", blocks = ["page_content"])]
pub struct TrendingTemplate<'a> {
    _phantom: std::marker::PhantomData<&'a ()>,
    gifs: Vec<Gif>,
    needs_title_update: bool,
}
impl<'a> TrendingTemplate<'a> {
    pub async fn new(context: &'a TrendingPageContext) -> Result<TrendingTemplate<'a>, Box<dyn Error>> {
        let needs_title_update = if get_hx_target(&context.route_headers).len() > 0 { true } else { false };

        let gifs = database::get_popular_gifs(0, 60).await?;

        Ok(TrendingTemplate {
            _phantom: std::marker::PhantomData,
            gifs,
            needs_title_update,
        })
    }
}
