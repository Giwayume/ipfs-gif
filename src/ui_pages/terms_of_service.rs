use std::error::Error;
use askama::Template;

use crate::router::routes::terms_of_service::TermsOfServicePageContext;

use crate::util::secrets::{ secrets_config };

#[derive(Template)]
#[template(path = "ui_pages/terms_of_service.html", blocks = ["page_content"])]
pub struct TermsOfServiceTemplate<'a> {
    arbitration_opt_out_email: &'a str,
}
impl<'a> TermsOfServiceTemplate<'a> {
    pub async fn new(_context: &'a TermsOfServicePageContext) -> Result<TermsOfServiceTemplate<'a>, Box<dyn Error>> {

        let arbitration_opt_out_email = &secrets_config().contact.arbitration_opt_out_email;

        Ok(TermsOfServiceTemplate { arbitration_opt_out_email })
    }
}

