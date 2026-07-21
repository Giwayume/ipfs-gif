use std::error::Error;
use askama::Template;

use crate::router::routes::moderation_queue::ModerationQueuePageContext;

use crate::util::secrets::{ secrets_config };

#[derive(Template)]
#[template(path = "ui_pages/moderation_queue.html", blocks = ["page_content"])]
pub struct ModerationQueueTemplate<'a> {
    arbitration_opt_out_email: &'a str,
}
impl<'a> ModerationQueueTemplate<'a> {
    pub async fn new(_context: &'a ModerationQueuePageContext) -> Result<ModerationQueueTemplate<'a>, Box<dyn Error>> {

        let arbitration_opt_out_email = &secrets_config().contact.arbitration_opt_out_email;

        Ok(ModerationQueueTemplate { arbitration_opt_out_email })
    }
}

