use std::io;
use std::error::Error;
use std::collections::HashSet;
use chrono::NaiveDateTime;
use sqlx::{
    QueryBuilder,
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
            JOIN tags_gifs tg ON tg.gif_id = g.id
            JOIN tags t ON t.id = tg.tag_id
            WHERE t.name = ? AND g.cid IS NOT NULL
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

pub async fn get_tags_by_gif_id(id: u64) -> Result<Vec<Tag>, Box<dyn Error>> {
    Ok(
        sqlx::query_as::<MySql, Tag>(r#"
            SELECT t.*
            FROM tags t
            JOIN tags_gifs tg ON tg.tag_id = t.id
            WHERE tg.gif_id = ?;
        "#)
            .bind(id)
            .fetch_all(get_pool())
            .await?
    )
}

async fn get_tag_by_name(name: &str) -> Result<Tag, Box<dyn Error>> {
    Ok(
        sqlx::query_as::<MySql, Tag>(r#"
            SELECT *
            FROM tags
            WHERE tags.name = ?
            LIMIT 1;
        "#)
            .bind(name)
            .fetch_one(get_pool())
            .await?
    )
}

pub async fn create_tag(name: &str) -> Result<u64, Box<dyn Error>> {
    let pool = get_pool();

    let check_exists_result = sqlx::query_as::<MySql, Tag>(r#"
        SELECT *
        FROM tags
        WHERE tags.name = ?
        LIMIT 1;
    "#)
        .bind(name)
        .fetch_optional(pool)
        .await;
    
    match check_exists_result {
        Ok(result) => {
            if let Some(existing_tag) = result {
                return Ok(existing_tag.id);
            }
        },
        _ => (),
    }

    let create_tag_result = sqlx::query(r#"
        INSERT INTO tags (name)
        VALUES (?)
        ON DUPLICATE KEY UPDATE
        name = name;
    "#)
        .bind(name)
        .execute(pool)
        .await;
    
    let tag_id = match create_tag_result {
        Ok(result) => {
            u64::try_from(result.last_insert_id()).unwrap_or_else(|_| 999)
        }
        Err(e) => {
            return Err(Box::new(e));
        }
    };

    let words = name.split_whitespace().collect::<Vec<&str>>();

    for word in words {
        let _ = sqlx::query(r#"
            INSERT INTO tag_tokens (tag_id, token)
            VALUES (?, ?)
            ON DUPLICATE KEY UPDATE
            token = token;
        "#)
            .bind(tag_id)
            .bind(word)
            .execute(pool)
            .await;
    }

    Ok(tag_id)
}

pub async fn add_tag_to_gif(gif_id: u64, tag_id: u64) -> Result<(), Box<dyn Error>> {
    let result = sqlx::query_as::<MySql, TagsGifs>(r#"
        INSERT INTO tags_gifs (gif_id, tag_id)
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

pub async fn remove_tag_from_gif(gif_id: u64, tag_name: &str) -> Result<(), Box<dyn Error>> {
    let tag = get_tag_by_name(tag_name).await?;

    let result = sqlx::query_as::<MySql, TagsGifs>(r#"
        DELETE FROM tags_gifs
        WHERE gif_id = ?
        AND tag_id = ?;
    "#)
        .bind(gif_id)
        .bind(tag.id)
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

fn find_first_tag_placement<'a>(
    input: &[&'a str],
    pattern: &[&'a str],
    used_idx: &[bool],
) -> Option<usize> {
    let m = pattern.len();
    if m == 0 || m > input.len() { return None; }

    'outer: for start in 0..=input.len().saturating_sub(m) {
        for j in 0..m {
            if used_idx[start + j] || input[start + j] != pattern[j] {
                continue 'outer;
            }
        }
        return Some(start);
    }
    None
}

fn mark_tag_used(used_idx: &mut [bool], start: usize, len: usize) {
    for i in start..start + len {
        used_idx[i] = true;
    }
}

/**
 * The purpose of this function is to try to match tags exactly to the user's input.
 * For example, if the user types "kita bocchi the rock", and we have the tags:
 * ["kita", "bocchi", "bocchi the rock"], this algorithm will try to match tags
 * to all words in the input without duplicates. In this case it will select the tags
 * ["kita", "bocchi the rock"] and leave out the tag ["bocchi"].
 * This resolves conflicts where the greater specificity helps pull more relevant
 * results than the shorter, generalized tags.
 */
fn pick_tags_greedy(
    search_words: Vec<String>,
    tag_candidates: Vec<(u64, String)>,
) -> Vec<u64> {
    if search_words.is_empty() {
        return vec![];
    }

    let input: Vec<&str> = search_words.iter().map(|s| s.as_str()).collect();
    let mut used_idx = vec![false; input.len()];
    let mut picked: Vec<u64> = Vec::new();

    // Collect tokenized tags once.
    let mut tags: Vec<(u64, String, Vec<String>)> = Vec::new();
    for (id, name) in tag_candidates {
        let tokens: Vec<String> = name
            .split_whitespace()
            .map(|t| t.to_string())
            .collect();
        if !tokens.is_empty() {
            tags.push((id, name.clone(), tokens));
        }
    }

    // Stage A: exact contiguous subsequence matches for full tag length.
    let mut stage_a: Vec<(u64, String, usize)> = tags
        .iter()
        .filter_map(|(id, name, tokens)| {
            let m = tokens.len();
            if m == 0 || m > input.len()  { return None; }

            let mut found = false;
            'outer: for start in 0..=input.len().saturating_sub(m) {
                for j in 0..m {
                    if input[start + j] != tokens[j] {
                        continue 'outer;
                    }
                }
                found = true;
                break;
            }
            found.then(|| (*id, name.clone(), m))
        })
        .collect();

    stage_a.sort_by(|a, b| {
        b.2.cmp(&a.2) // token-count desc
            .then_with(|| b.1.len().cmp(&a.1.len())) // name length desc
            .then_with(|| a.0.cmp(&b.0)) // id asc
    });

    for (id, name, _m) in stage_a {
        let tokens: Vec<&str> = name.split_whitespace().collect();
        if let Some(start) = find_first_tag_placement(&input, &tokens, &used_idx) {
            mark_tag_used(&mut used_idx, start, tokens.len());
            picked.push(id);
        }
        if used_idx.iter().all(|&u| u) {
            break;
        }
    }

    // Stage B: partial matches. We allow selecting shorter tags that match
    // contiguously in the query, but only at positions not already used.
    // If query is "hello" and tag "hello world" exists, we’ll select "hello"
    // (if it exists) rather than forcing the longer tag.
    let mut stage_b: Vec<(u64, String, usize)> = tags
        .iter()
        .filter_map(|(id, name, tokens)| {
            let m = tokens.len();
            if m == 0 { return None; }

            // Partial means: there exists *some* contiguous subsequence of this tag
            // (of length >= 1) that matches a contiguous subsequence in input.
            // Equivalent: the tag tokens themselves may not all match, but at least
            // one token-prefix (or token-subsequence) will.
            let mut any = false;

            'outer: for start in 0..input.len() {
                // try increasing length; stop when it exceeds tag length
                for l in 1..=m {
                    if start + l > input.len() { break; }
                    // compare input[start..start+l] to tag[0..l]
                    // (using prefix of tag keeps it simple & consistent)
                    let matches_prefix = (0..l).all(|j| input[start + j] == tokens[j]);
                    if matches_prefix {
                        any = true;
                        break 'outer;
                    }
                }
            }

            any.then(|| (*id, name.clone(), m))
        })
        .collect();

    stage_b.sort_by(|a, b| {
        // Prefer longer tags first even in partial mode.
        b.2.cmp(&a.2)
            .then_with(|| b.1.len().cmp(&a.1.len()))
            .then_with(|| a.0.cmp(&b.0))
    });

    for (id, name, _m) in stage_b {
        if used_idx.iter().all(|&u| u) {
            break;
        }

        // Try to place the longest prefix of the tag that can fit on unused indices.
        let tokens: Vec<&str> = name.split_whitespace().collect();
        for l in (1..=tokens.len()).rev() {
            let prefix = &tokens[..l];
            if let Some(start) = find_first_tag_placement(&input, prefix, &used_idx) {
                mark_tag_used(&mut used_idx, start, l);
                picked.push(id);
                break;
            }
        }
    }

    picked
}

pub async fn search_by_tags(search: &str, limit: u32) -> Result<Vec<Gif>, Box<dyn Error>> {
    let allowed = regex::Regex::new(r"[^A-Za-z0-9]+").unwrap();
    let punctuation = regex::Regex::new(r"['\-.]").unwrap();
    let search_words = allowed.split(
        &punctuation.replace_all(search, " ")
    )
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string().to_ascii_lowercase())
        .collect::<Vec<String>>();
    
    let unique_search_words = search_words
        .iter()
        .map(|s| s.as_str())
        .collect::<HashSet<&str>>()
        .into_iter()
        .collect::<Vec<&str>>();
    
    if unique_search_words.len() == 0 {
        return Ok(Vec::new());
    }

    let pool = get_pool();
    
    // Find tags that match all words found in the search query (preferred)
    // And tags that only match some of the words as a backup.
    let mut qb = QueryBuilder::<MySql>::new(r#"
        SELECT
            t.id,
            t.name,
            tt_tot.total_token_cnt,
            tt_in.full_match_cnt
        FROM tags t
        JOIN (
            SELECT tag_id, COUNT(DISTINCT token) AS total_token_cnt
            FROM tag_tokens
            GROUP BY tag_id
        ) AS tt_tot ON tt_tot.tag_id = t.id
        LEFT JOIN (
            SELECT tag_id, COUNT(DISTINCT token) AS full_match_cnt
            FROM tag_tokens
            WHERE token IN (
    "#);

    let mut separated = qb.separated(", ");
    for word in unique_search_words {
        separated.push_bind(word);
    }

    qb.push(r#"
            )
            GROUP BY tag_id
        ) AS tt_in ON tt_in.tag_id = t.id
        WHERE
        COALESCE(tt_in.full_match_cnt, 0) >= 1
        ORDER BY
            (COALESCE(tt_in.full_match_cnt, 0) = tt_tot.total_token_cnt) DESC,
            COALESCE(tt_in.full_match_cnt, 0) DESC,
            tt_tot.total_token_cnt DESC,
            t.id ASC
        LIMIT 100;
    "#);

    let query = qb.build_query_as::<(u64, String)>();
    let tag_candidates = match query.fetch_all(pool).await {
        Ok(tag_candidates) => tag_candidates,
        Err(e) => {
            return Err(Box::new(e));
        }
    };

    let picked_tag_ids = pick_tags_greedy(search_words, tag_candidates);

    if picked_tag_ids.len() == 0 {
        return Ok(Vec::new());
    }

    // Find GIFs that have the specified picked_tag_ids
    qb = QueryBuilder::<MySql>::new(r#"
        SELECT
            g.*,
            COUNT(DISTINCT t.id) AS matched_tag_count
        FROM gifs g
        JOIN tags_gifs tg ON tg.gif_id = g.id
        JOIN tags t ON t.id = tg.tag_id
        WHERE (
    "#);

    separated = qb.separated(" OR ");
    for tag_id in picked_tag_ids {
        separated.push("t.id = ");
        separated.push_bind_unseparated(tag_id);
    }

    qb.push(r#"
        ) AND g.cid IS NOT NULL
        GROUP BY g.id
        HAVING COUNT(DISTINCT t.id) >= 1
        ORDER BY matched_tag_count DESC, g.popularity DESC
        LIMIT 
    "#);
    qb.push_bind(limit);

    let query = qb.build_query_as::<Gif>();
    let result = query.fetch_all(pool).await;

    match result {
        Ok(gifs) => {
            Ok(gifs)
        }
        Err(e) => {
            Err(Box::new(e))
        }
    }
}
