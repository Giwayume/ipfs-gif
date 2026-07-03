use std::error::Error;
use std::marker::PhantomData;
use askama::Template;

use crate::router::routes::dcma::DcmaPageContext;

use crate::util::secrets::{ secrets_config };

#[derive(Template)]
#[template(path = "ui_pages/dcma.html", blocks = ["page_content"])]
pub struct DcmaTemplate<'a> {
    dcma_email: &'a str,
}
impl<'a> DcmaTemplate<'a> {
    pub async fn new(_context: &'a DcmaPageContext) -> Result<DcmaTemplate<'a>, Box<dyn Error>> {

        let dcma_email = &secrets_config().contact.dcma_email;

        Ok(DcmaTemplate { dcma_email })
    }
}

