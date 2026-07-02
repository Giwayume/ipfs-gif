use std::io;
use std::error::Error;
use chrono::NaiveDateTime;
use sqlx::{
    FromRow,
    MySql,
};
use super::get_pool;

#[allow(unused)]
#[derive(Debug, Default, FromRow)]
pub struct Gif {
    pub id: u64,
    pub cid: String,
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
            ORDER BY gifs.popularity DESC
            LIMIT ?
            OFFSET ?;
        "#)
            .bind(length)
            .bind(start)
            .fetch_all(get_pool())
            .await?
    )
}

pub async fn get_gif_by_cid(cid: &str) -> Result<Gif, Box<dyn Error>> {
    Ok(
        sqlx::query_as::<MySql, Gif>(r#"
            SELECT gifs.*
            FROM gifs
            WHERE gifs.cid=?
            LIMIT 1;
        "#)
            .bind(cid)
            .fetch_one(get_pool())
            .await?
    )
}

pub async fn create_gif(
    gif: Gif
) -> Result<u64, Box<dyn Error + Send + Sync>> {
    let pool = get_pool();

    let result = sqlx::query(r#"
        INSERT INTO gifs (
            cid, upload_time, uploader_public_key, uploader_ip_address,
            filename, description, popularity, width, height, size, frames
        )
        VALUES (?, NOW(), ?, ?, ?, ?, 0, ?, ?, ?, ?)
    "#)
        .bind(gif.cid)
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
