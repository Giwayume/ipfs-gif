use std::error::Error;
use askama::Template;
use garde::{ Report };

use crate::router::routes::explore::ExplorePageContext;
use crate::router::{ get_hx_target };
use crate::database::{ self, Tag };
use crate::util::format;

#[derive(Template)]
#[template(path = "ui_pages/explore.html", blocks = ["page_content"])]
pub struct ExploreTemplate<'a> {
    _phantom: std::marker::PhantomData<&'a ()>,
    tags: Vec<(String, String)>,
    needs_title_update: bool,
}
impl<'a> ExploreTemplate<'a> {
    pub async fn new(context: &'a ExplorePageContext) -> Result<ExploreTemplate<'a>, Box<dyn Error>> {
        let needs_title_update = if get_hx_target(&context.route_headers).len() > 0 { true } else { false };

        let tags = database::get_popular_tags(0, 60).await?
            .into_iter()
            .map(|t| (format::to_kebab_case(&t.name), t.name))
            .collect::<Vec<(String, String)>>();

        Ok(ExploreTemplate {
            _phantom: std::marker::PhantomData,
            tags,
            needs_title_update,
        })
    }
}
