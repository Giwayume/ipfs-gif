use std::error::Error;
use askama::Template;

use crate::router::routes::home::HomePageContext;
use crate::database::{ self, Gif };

#[derive(Template)]
#[template(path = "ui_pages/home.html", blocks = ["page_content"])]
pub struct HomeTemplate<'a> {
    _phantom: std::marker::PhantomData<&'a ()>,
    popular_gifs: Vec<Gif>,
}
impl<'a> HomeTemplate<'a> {
    pub async fn new(_context: &'a HomePageContext) -> Result<HomeTemplate<'a>, Box<dyn Error>> {
        let popular_gifs = database::get_popular_gifs(0, 20).await?;

        Ok(HomeTemplate { _phantom: std::marker::PhantomData, popular_gifs })
    }
}