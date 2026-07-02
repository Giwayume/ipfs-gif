use std::error::Error;
use std::marker::PhantomData;
use askama::Template;

use crate::router::routes::gif::GifPageContext;
use crate::database::{ self, Gif };
use crate::ui_primitives::alert::AlertTemplate;
use crate::util::format;

#[derive(Template)]
#[template(path = "ui_pages/gif.html", blocks = ["page_content"])]
pub struct GifTemplate<'a> {
    active_page: &'a str,
    already_uploaded_alert: Option<AlertTemplate<'a>>,
    gif: Gif,
    tags: Vec<(String, String)>,
}
impl<'a> GifTemplate<'a> {
    pub async fn new(context: &'a GifPageContext) -> Result<GifTemplate<'a>, Box<dyn Error>> {
        let active_page: &str = "home";

        let already_uploaded_alert = if let Some(_) = context.route_query.get("already-uploaded") {
            Some(AlertTemplate {
                variant: "info",
                message_html: String::from("<p>Someone else already uploaded this GIF! We've taken you to it.</p>"),
            })
        } else {
            None
        };

        let gif = database::get_gif_by_cid(&context.params.cid).await?;

        let tags = database::get_tags_by_gif_id(gif.id).await?
            .into_iter()
            .map(|t| (format::to_kebab_case(&t.name), t.name))
            .collect::<Vec<(String, String)>>();

        Ok(GifTemplate { active_page, already_uploaded_alert, gif, tags })
    }
}

