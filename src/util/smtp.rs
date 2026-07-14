/**
 * SMTP client setup, for sending notification & password reset emails.
 */

use std::error::Error;
use std::fs;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{ Message, SmtpTransport, Transport };
use tokio::sync::OnceCell;

use crate::util::secrets::secrets_config;

pub static MAILER: OnceCell<SmtpTransport> = OnceCell::const_new();

pub fn init_mailer() {
    let config = secrets_config();
    let username = &config.smtp.username;
    let password = &config.smtp.password;
    let relay_server_name = &config.smtp.relay_server_name;

    tracing::info!("SMTP username: {}", username);
    tracing::info!("SMTP relay server name: {}", relay_server_name);

    let credentials = Credentials::new(username.to_owned(), password.to_owned());

    let mailer = SmtpTransport::starttls_relay(&relay_server_name)
        .unwrap()
        .credentials(credentials)
        .build();
    
    MAILER.set(mailer).expect("Mailer already initialized.");
}

pub fn send_email(to: &str, from: &str, subject: String, body: String, content_type: ContentType) -> Result<(), Box<dyn Error + Send + Sync>> {
    let email = Message::builder()
        .from(format!("Interplanetary GIFs <{}>", from).parse().unwrap())
        .to(to.parse().unwrap())
        .subject(subject)
        .header(content_type)
        .body(body)
        .unwrap();
    let mailer = MAILER.get().expect("Mailer is not initialized.");
    match mailer.send(&email) {
        Ok(_) => {
            Ok(())
        },
        Err(e) => {
            Err(Box::new(e))
        },
    }
}
