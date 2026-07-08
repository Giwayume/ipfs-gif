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
use super::gifs::{ GifModerationStatus };

#[derive(Clone, Debug, Default, Display, EnumString, PartialEq, Serialize, Type)]
#[sqlx(type_name = "quarantine_scan_result")]
#[sqlx(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ModerationReportType {
    Dcma,
    Illegal,
    Doxxing,
    Gore,
    Sexual,
    #[default]
    Other,
}

#[derive(Clone, Debug, Default, FromRow)]
pub struct ModerationReport {
    pub id: u64,
    pub gif_id: u64,
    pub report_time: NaiveDateTime,
    pub report_type: ModerationReportType,
    pub copyright_holder_name: String,
    pub reporter_public_key: String,
    pub reporter_ip_address: String,
    pub reporter_name: String,
    pub reporter_mailing_address: String,
    pub reporter_phone: String,
    pub reporter_email: String,
    pub reporter_attestation: String,
}

#[derive(Clone, Debug, Default, FromRow)]
pub struct ModerationCounterClaim {
    pub id: u64,
    pub report_id: u64,
    pub counter_claim_time: NaiveDateTime,
    pub counter_claimant_public_key: String,
    pub counter_claimant_ip_address: String,
    pub counter_claimant_name: String,
    pub counter_claimant_mailing_address: String,
    pub counter_claimant_phone: String,
    pub counter_claimant_email: String,
    pub counter_claimant_attestation: String,
}

pub async fn get_moderation_reports_by_gif_id(
    gif_id: u64
) -> Result<Vec<ModerationReport>, Box<dyn Error>> {
    Ok(
        sqlx::query_as::<MySql, ModerationReport>(r#"
            SELECT moderation_reports.*
            FROM moderation_reports
            WHERE moderation_reports.gif_id=?
            LIMIT 10
        "#)
            .bind(gif_id)
            .fetch_all(get_pool())
            .await?
    )
}

pub async fn create_moderation_report(
    moderation_report: ModerationReport
) -> Result<u64, Box<dyn Error + Send + Sync>> {
    let pool = get_pool();

    let gif_id = moderation_report.gif_id;

    let existing_report_count: i64 = sqlx::query_scalar(r#"
        SELECT COUNT(DISTINCT reporter_ip_address)
        FROM moderation_reports
        WHERE report_type=?
    "#)
        .bind(&moderation_report.report_type)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|_| 0);

    let gif_moderation_status = match &moderation_report.report_type {
        ModerationReportType::Dcma => Some(GifModerationStatus::DcmaTakedownNotice),
        ModerationReportType::Illegal => if existing_report_count >= 50 { Some(GifModerationStatus::IllegalRemoved) } else { None },
        ModerationReportType::Doxxing => if existing_report_count >= 50 { Some(GifModerationStatus::DoxxingRemoved) } else { None },
        ModerationReportType::Gore => if existing_report_count >= 50 { Some(GifModerationStatus::GoreRemoved) } else { None },
        ModerationReportType::Sexual => if existing_report_count >= 50 { Some(GifModerationStatus::SexualRemoved) } else { None },
        ModerationReportType::Other => None,
    };

    let result = sqlx::query(r#"
        INSERT INTO moderation_reports (
            gif_id, report_time, report_type, copyright_holder_name, reporter_public_key, reporter_ip_address,
            reporter_name, reporter_mailing_address, reporter_phone, reporter_email, reporter_attestation
        )
        VALUES (?, NOW(), ?, ?, ?, ?, ?, ?, ?, ?, ?)
    "#)
        .bind(moderation_report.gif_id)
        .bind(moderation_report.report_type)
        .bind(moderation_report.copyright_holder_name)
        .bind(moderation_report.reporter_public_key)
        .bind(moderation_report.reporter_ip_address)
        .bind(moderation_report.reporter_name)
        .bind(moderation_report.reporter_mailing_address)
        .bind(moderation_report.reporter_phone)
        .bind(moderation_report.reporter_email)
        .bind(moderation_report.reporter_attestation)
        .execute(pool)
        .await;
    
    let moderation_report_id = match result {
        Ok(result) => u64::try_from(result.last_insert_id()).unwrap_or_else(|_| 999),
        Err(e) => {
            return Err(Box::new(e));
        }
    };

    if let Some(gif_moderation_status) = gif_moderation_status {
        let result = sqlx::query(r#"
            UPDATE gifs
            SET moderation_status=?
            WHERE gifs.id=?
        "#)
            .bind(gif_moderation_status)
            .bind(gif_id)
            .execute(pool)
            .await;
        
        if let Err(e) = result {
            return Err(Box::new(e));
        }
    }

    Ok(moderation_report_id)
}

pub async fn create_moderation_counter_claim(
    moderation_counter_claim: ModerationCounterClaim
) -> Result<u64, Box<dyn Error + Send + Sync>> {
    let pool = get_pool();

    let moderation_report = match sqlx::query_as::<MySql, ModerationReport>(r#"
        SELECT moderation_reports.*
        FROM moderation_reports
        WHERE moderation_reports.id=?
        LIMIT 1
    "#)
        .bind(moderation_counter_claim.report_id)
        .fetch_one(get_pool())
        .await
    {
        Ok(moderation_report) => moderation_report,
        Err(e) => {
            return Err(Box::new(e));
        }
    };

    let gif_moderation_status = match &moderation_report.report_type {
        ModerationReportType::Dcma => Some(GifModerationStatus::DcmaCounterClaim),
        _ => None,
    };

    let result = sqlx::query(r#"
        INSERT INTO moderation_counter_claims (
            report_id, counter_claim_time, counter_claimant_public_key, counter_claimant_ip_address,
            counter_claimant_name, counter_claimant_mailing_address, counter_claimant_phone, counter_claimant_email, counter_claimant_attestation
        )
        VALUES (?, NOW(), ?, ?, ?, ?, ?, ?, ?)
    "#)
        .bind(moderation_counter_claim.report_id)
        .bind(moderation_counter_claim.counter_claimant_public_key)
        .bind(moderation_counter_claim.counter_claimant_ip_address)
        .bind(moderation_counter_claim.counter_claimant_name)
        .bind(moderation_counter_claim.counter_claimant_mailing_address)
        .bind(moderation_counter_claim.counter_claimant_phone)
        .bind(moderation_counter_claim.counter_claimant_email)
        .bind(moderation_counter_claim.counter_claimant_attestation)
        .execute(pool)
        .await;
    
    let moderation_counter_claim_id = match result {
        Ok(result) => u64::try_from(result.last_insert_id()).unwrap_or_else(|_| 999),
        Err(e) => {
            return Err(Box::new(e));
        }
    };

    if let Some(gif_moderation_status) = gif_moderation_status {
        let result = sqlx::query(r#"
            UPDATE gifs
            SET moderation_status = ?
            WHERE gifs.id = ?
        "#)
            .bind(gif_moderation_status)
            .bind(moderation_report.gif_id)
            .execute(pool)
            .await;
        
        if let Err(e) = result {
            return Err(Box::new(e));
        }
    }

    Ok(moderation_counter_claim_id)
}

