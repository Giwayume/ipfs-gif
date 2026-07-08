use std::fs;
use serde::Deserialize;
use sqlx::{
    MySqlPool,
};
use tokio::sync::OnceCell;
use urlencoding::encode;

#[derive(Deserialize)]
struct SecretsConfig {
    database: SecretsConfigDatabase,
}

#[derive(Deserialize)]
struct SecretsConfigDatabase {
    host: String,
    port: u16,
    user: String,
    password: String,
}

#[derive(Debug, PartialEq)]
pub enum QueryOrder {
    Asc,
    Desc,
}

pub static POOL: OnceCell<MySqlPool> = OnceCell::const_new();

pub async fn init_pool() {
    let secrets_toml = fs::read_to_string(format!(
        "{}/config/secrets.toml",
        env!("CARGO_MANIFEST_DIR")
    )).expect("Failed to read secrets.toml file.");
    let config: SecretsConfig = toml::from_str(&secrets_toml)
        .expect("Failed to parse secrets.toml file.");

    tracing::info!("Database host: {}", config.database.host);
    tracing::info!("Database port: {}", config.database.port);
    tracing::info!("Database user: {}", config.database.user);

    let database_url: &str = &format!(
        "mysql://{}:{}@{}:{}/ipfs_gif",
        config.database.user.as_str(), encode(config.database.password.as_str()),
        config.database.host.as_str(), config.database.port,
    );

    let pool = MySqlPool::connect(database_url)
        .await
        .expect("Failed to create pool.");

    POOL.set(pool).expect("Pool already initialized.");
}

pub fn get_pool() -> &'static MySqlPool {
    POOL.get().expect("Pool is not initialized.")
}

pub mod initialize;

pub mod gifs;
pub use gifs::Gif;
pub use gifs::QuarantineScanResult;
pub use gifs::GifModerationStatus;
pub use gifs::get_popular_gifs;
pub use gifs::get_gif_by_cid;
pub use gifs::get_next_quarantined_gif;
pub use gifs::create_gif;
pub use gifs::update_gif;
pub use gifs::update_gif_cid;
pub use gifs::delete_gif_by_id;
pub use gifs::delete_old_quarantine_gifs;
pub use gifs::update_gif_quarantine_scan_result;
pub use gifs::increment_gif_popularity;

pub mod moderation_report;
pub use moderation_report::ModerationReport;
pub use moderation_report::ModerationReportType;
pub use moderation_report::ModerationCounterClaim;
pub use moderation_report::get_moderation_reports_by_gif_id;
pub use moderation_report::create_moderation_report;
pub use moderation_report::create_moderation_counter_claim;

pub mod tags;
pub use tags::Tag;
pub use tags::get_gifs_by_tag;
pub use tags::get_tags_by_gif_id;
pub use tags::get_popular_tags;
pub use tags::create_tag;
pub use tags::add_tag_to_gif;
pub use tags::remove_tag_from_gif;
pub use tags::search_by_tags;