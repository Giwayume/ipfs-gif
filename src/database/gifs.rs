use std::io;
use std::error::Error;
use chrono::NaiveDateTime;
use sqlx::{
    FromRow,
    MySql,
    Type,
};
use serde::Serialize;
use strum_macros::{ Display, EnumString };
use super::get_pool;

#[derive(Clone, Debug, Default, Display, EnumString, PartialEq, Serialize, Type)]
#[sqlx(type_name = "quarantine_scan_result")]
#[sqlx(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum QuarantineScanResult {
    MissingImage,
    ImageParseFailed,
    ScanFailed,
    IpfsTransferFailed,
    IpfsDuplicate,
    #[default]
    None,
}

#[allow(unused)]
#[derive(Clone, Debug, Default, FromRow)]
pub struct Gif {
    pub id: u64,
    pub cid: Option<String>,
    pub quarantine_id: String,
    pub quarantine_scan_result: QuarantineScanResult,
    pub upload_time: NaiveDateTime,
    pub uploader_public_key: String,
    pub uploader_ip_address: String,
    pub filename: String,
    pub description: String,
    pub popularity: u64,
    pub width: u32,
    pub height: u32,
    pub size: u32,
    pub frames: u32,
}

pub async fn get_popular_gifs(start: u64, length: u64) -> Result<Vec<Gif>, Box<dyn Error>> {
    Ok(
        sqlx::query_as::<MySql, Gif>(r#"
            SELECT gifs.*
            FROM gifs
            WHERE gifs.cid IS NOT NULL
            ORDER BY gifs.popularity DESC
            LIMIT ?
            OFFSET ?
        "#)
            .bind(length)
            .bind(start)
            .fetch_all(get_pool())
            .await?
    )
}

pub async fn get_gif_by_cid(cid: &str) -> Result<Gif, Box<dyn Error>> {
    let result = if cid.starts_with("qt-") {
        sqlx::query_as::<MySql, Gif>(r#"
            SELECT gifs.*
            FROM gifs
            WHERE gifs.quarantine_id=?
            LIMIT 1
        "#)
            .bind(cid)
            .fetch_one(get_pool())
            .await
    } else {
        sqlx::query_as::<MySql, Gif>(r#"
            SELECT gifs.*
            FROM gifs
            WHERE gifs.cid=?
            LIMIT 1
        "#)
            .bind(cid)
            .fetch_one(get_pool())
            .await
    };

    match result {
        Ok(result) => Ok(result),
        Err(e) => Err(Box::new(e)),
    }
}

pub async fn get_next_quarantined_gif() -> Result<Gif, Box<dyn Error>> {
    let result = sqlx::query_as::<MySql, Gif>(r#"
        SELECT gifs.*
        FROM gifs
        WHERE gifs.cid IS NULL AND gifs.quarantine_scan_result = "none"
        LIMIT 1
    "#)
        .fetch_one(get_pool())
        .await;
    
    match result {
        Ok(result) => Ok(result),
        Err(e) => Err(Box::new(e)),
    }
}

pub async fn create_gif(
    gif: Gif
) -> Result<u64, Box<dyn Error + Send + Sync>> {
    let pool = get_pool();

    let result = sqlx::query(r#"
        INSERT INTO gifs (
            cid, quarantine_id, upload_time, uploader_public_key, uploader_ip_address,
            filename, description, popularity, width, height, size, frames
        )
        VALUES (?, ?, NOW(), ?, ?, ?, ?, 0, ?, ?, ?, ?)
    "#)
        .bind(gif.cid)
        .bind(gif.quarantine_id)
        .bind(gif.uploader_public_key)
        .bind(gif.uploader_ip_address)
        .bind(gif.filename)
        .bind(gif.description)
        .bind(gif.width)
        .bind(gif.height)
        .bind(gif.size)
        .bind(gif.frames)
        .execute(pool)
        .await;
    
    match result {
        Ok(result) => {
            Ok(u64::try_from(result.last_insert_id()).unwrap_or_else(|_| 999))
        }
        Err(e) => {
            Err(Box::new(e))
        }
    }
}

pub async fn update_gif(
    gif: Gif
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let result = sqlx::query_as::<MySql, Gif>(r#"
        UPDATE gifs
        SET filename=?, description=?
        WHERE id=?
        LIMIT 1
    "#)
        .bind(gif.filename)
        .bind(gif.description)
        .bind(gif.id)
        .fetch_optional(get_pool())
        .await;

    match result {
        Ok(_) => {
            Ok(())
        }
        Err(e) => {
            Err(Box::new(e))
        }
    }
}

#[allow(unused)]
pub async fn delete_gif_by_id(id: u64) -> Result<(), Box<dyn Error + Send + Sync>> {
    let result = sqlx::query_as::<MySql, Gif>(r#"
        DELETE FROM gifs
        WHERE id=?
        LIMIT 1
    "#)
        .bind(id)
        .fetch_optional(get_pool())
        .await;

    match result {
        Ok(_) => {
            Ok(())
        }
        Err(e) => {
            Err(Box::new(e))
        }
    }
}

pub async fn delete_old_quarantine_gifs() -> Result<(), Box<dyn Error + Send + Sync>> {
    let _ = sqlx::query_as::<MySql, Gif>(r#"
        DELETE FROM gifs
        WHERE gifs.upload_time < NOW() - INTERVAL 1 HOUR AND gifs.cid IS NULL;
    "#)
        .fetch_optional(get_pool())
        .await;
    
    let _ = sqlx::query_as::<MySql, Gif>(r#"
        DELETE tg
        FROM tags_gifs tg
        LEFT JOIN gifs g ON g.id = tg.gif_id
        WHERE g.id IS NULL;
    "#)
        .fetch_optional(get_pool())
        .await;

    Ok(())
}

pub async fn update_gif_quarantine_scan_result(id: u64, quarantine_scan_result: QuarantineScanResult) -> Result<(), Box<dyn Error + Send + Sync>> {
    let result = sqlx::query_as::<MySql, Gif>(r#"
        UPDATE gifs
        SET quarantine_scan_result=?
        WHERE id=?
        LIMIT 1
    "#)
        .bind(quarantine_scan_result)
        .bind(id)
        .fetch_optional(get_pool())
        .await;

    match result {
        Ok(_) => {
            Ok(())
        }
        Err(e) => {
            Err(Box::new(e))
        }
    }
}

pub async fn update_gif_cid(id: u64, cid: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    let result = sqlx::query_as::<MySql, Gif>(r#"
        UPDATE gifs
        SET cid=?
        WHERE id=?
        LIMIT 1
    "#)
        .bind(cid)
        .bind(id)
        .fetch_optional(get_pool())
        .await;

    match result {
        Ok(_) => {
            Ok(())
        }
        Err(e) => {
            Err(Box::new(e))
        }
    }
}

pub async fn increment_gif_popularity(
    cid: String
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let result = sqlx::query_as::<MySql, Gif>(r#"
        UPDATE gifs
        SET popularity = popularity + 1
        WHERE cid=?
        LIMIT 1
    "#)
        .bind(cid)
        .fetch_optional(get_pool())
        .await;
    
    match result {
        Ok(_) => {
            Ok(())
        }
        Err(e) => {
            Err(Box::new(e))
        }
    }
}
