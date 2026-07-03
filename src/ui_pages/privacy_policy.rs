use std::error::Error;
use askama::Template;

use crate::router::routes::privacy_policy::PrivacyPolicyPageContext;

#[derive(Template)]
#[template(path = "ui_pages/privacy_policy.html", blocks = ["page_content"])]
pub struct PrivacyPolicyTemplate<'a> {
    _phantom: std::marker::PhantomData<&'a ()>,
}
impl<'a> PrivacyPolicyTemplate<'a> {
    pub async fn new(_context: &'a PrivacyPolicyPageContext) -> Result<PrivacyPolicyTemplate<'a>, Box<dyn Error>> {
        Ok(PrivacyPolicyTemplate {
            _phantom: std::marker::PhantomData,
        })
    }
}

