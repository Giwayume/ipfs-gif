use std::io;
use std::error::Error;
use chrono::NaiveDateTime;
use sqlx::{
    FromRow,
    MySql,
};
use super::get_pool;
use super::gifs::Gif;

#[allow(unused)]
#[derive(Debug, Default, FromRow)]
pub struct Tag {
    pub id: u64,
    pub name: String,
}

#[allow(unused)]
#[derive(Debug, Default, FromRow)]
pub struct TagsGifs {
    pub gif_id: u64,
    pub tag_id: u64,
}

pub async fn get_gifs_by_tag(tag_name: &str, start: u64, length: u64) -> Result<Vec<Gif>, Box<dyn Error>> {
    Ok(
        sqlx::query_as::<MySql, Gif>(r#"
            SELECT g.*
            FROM gifs g
            JOIN gifs_tags gt ON gt.gif_id = s.id
            JOIN tags t ON t.id = gt.tag_id
            WHERE t.name = ?
            ORDER BY g.popularity DESC
            LIMIT ?
            OFFSET ?;
        "#)
            .bind(tag_name)
            .bind(length)
            .bind(start)
            .fetch_all(get_pool())
            .await?
    )
}

pub async fn get_tags_by_gif_cid(cid: &str) -> Result<Vec<Tag>, Box<dyn Error>> {
    Ok(
        sqlx::query_as::<MySql, Tag>(r#"
            SELECT t.*
            FROM tags t
            JOIN gifs_tags gt ON gt.tag_id = t.id
            JOIN gifs g ON g.id = gt.song_id
            WHERE g.cid = ?; 
        "#)
            .bind(cid)
            .fetch_all(get_pool())
            .await?
    )
}

pub async fn create_tag(name: &str) -> Result<u64, Box<dyn Error>> {
    let pool = get_pool();

    let result = sqlx::query(r#"
        INSERT INTO tags (name)
        VALUES (?)
        ON DUPLICATE KEY UPDATE
        name = name;
    "#)
        .bind(name)
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

pub async fn add_tag_to_gif(gif_id: u64, tag_id: u64) -> Result<(), Box<dyn Error>> {
    let result = sqlx::query_as::<MySql, TagsGifs>(r#"
        INSERT INTO gifs_tags (gif_id, tag_id)
        VALUES (?, ?)
        ON DUPLICATE KEY UPDATE
        gif_id = gif_id;
    "#)
        .bind(gif_id)
        .bind(tag_id)
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

pub async fn remove_tag_from_gif(gif_id: u64, tag_id: u64) -> Result<(), Box<dyn Error>> {
    let result = sqlx::query_as::<MySql, TagsGifs>(r#"
        DELETE FROM gifs_tags
        WHERE gif_id = ?
        AND tag_id = ?;
    "#)
        .bind(gif_id)
        .bind(tag_id)
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

