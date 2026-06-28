use std::error::Error;
use std::marker::PhantomData;
use askama::Template;

use crate::router::routes::privacy_policy::PrivacyPolicyPageContext;

#[derive(Template)]
#[template(path = "ui_pages/privacy_policy.html", blocks = ["page_content"])]
pub struct PrivacyPolicyTemplate<'a> {
    active_page: &'a str,
}
impl<'a> PrivacyPolicyTemplate<'a> {
    pub async fn new(context: &'a PrivacyPolicyPageContext) -> Result<PrivacyPolicyTemplate<'a>, Box<dyn Error>> {
        let active_page: &str = "home";

        Ok(PrivacyPolicyTemplate { active_page })
    }
}

