use std::error::Error;
use sqlx::{
    FromRow,
    MySql,
};

use super::get_pool;

#[derive(Debug, Default, Clone, FromRow)]
struct IgnoreDataType {}

#[allow(unused)]
async fn create_gifs_table() -> Result<(), Box<dyn Error + Send + Sync>> {
    let result = sqlx::query_as::<MySql, IgnoreDataType>(r#"
        CREATE TABLE IF NOT EXISTS gifs (
            id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
            cid VARCHAR(128) DEFAULT '',
            upload_time DATETIME DEFAULT NOW(),
            uploader_public_key VARCHAR(64) DEFAULT '',
            uploader_ip_address VARCHAR(64) DEFAULT '',
            filename VARCHAR(256) DEFAULT '',
            description VARCHAR(256) DEFAULT '',
            popularity BIGINT UNSIGNED NOT NULL DEFAULT 0,
            width INTEGER UNSIGNED NOT NULL DEFAULT 0,
            height INTEGER UNSIGNED NOT NULL DEFAULT 0,
            size INTEGER UNSIGNED NOT NULL DEFAULT 0,
            frames INTEGER UNSIGNED NOT NULL DEFAULT 0
        ) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
    "#)
        .fetch_optional(get_pool())
        .await;
    match result {
        Ok(_) => Ok(()),
        Err(error) => {
            tracing::error!("Error creating gifs table {:?}", error);
            Err(Box::new(error))
        }
    }
}

#[allow(unused)]
async fn create_tags_table() -> Result<(), Box<dyn Error + Send + Sync>> {
    let result = sqlx::query_as::<MySql, IgnoreDataType>(r#"
        CREATE TABLE IF NOT EXISTS tags (
            id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
            name VARCHAR(256) NOT NULL UNIQUE
        ) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
    "#)
        .fetch_optional(get_pool())
        .await;
    match result {
        Ok(_) => Ok(()),
        Err(error) => {
            tracing::error!("Error creating tags table {:?}", error);
            Err(Box::new(error))
        }
    }
}

#[allow(unused)]
async fn create_tags_gifs_table() -> Result<(), Box<dyn Error + Send + Sync>> {
    let result = sqlx::query_as::<MySql, IgnoreDataType>(r#"
        CREATE TABLE IF NOT EXISTS tags_gifs (
            gif_id BIGINT UNSIGNED NOT NULL REFERENCES gifs(id) ON DELETE CASCADE,
            tag_id BIGINT UNSIGNED NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY (gif_id, tag_id)
        ) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
    "#)
        .fetch_optional(get_pool())
        .await;
    match result {
        Ok(_) => Ok(()),
        Err(error) => {
            tracing::error!("Error creating tags_gifs table {:?}", error);
            Err(Box::new(error))
        }
    }
}

#[allow(unused)]
pub async fn create_all_tables() {
    create_gifs_table().await;
    create_tags_table().await;
    create_tags_gifs_table().await;
}
