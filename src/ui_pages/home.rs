use std::error::Error;
use askama::Template;

use crate::router::routes::home::HomePageContext;
use crate::router::{ get_hx_target };
use crate::database::{ self, Gif };

#[derive(Template)]
#[template(path = "ui_pages/home.html", blocks = ["page_content"])]
pub struct HomeTemplate<'a> {
    _phantom: std::marker::PhantomData<&'a ()>,
    needs_title_update: bool,
    popular_gifs: Vec<Gif>,
}
impl<'a> HomeTemplate<'a> {
    pub async fn new(context: &'a HomePageContext) -> Result<HomeTemplate<'a>, Box<dyn Error>> {
        let needs_title_update = if get_hx_target(&context.route_headers).len() > 0 { true } else { false };

        let popular_gifs = database::get_popular_gifs(0, 20).await?;

        Ok(HomeTemplate { _phantom: std::marker::PhantomData, needs_title_update, popular_gifs })
    }
}