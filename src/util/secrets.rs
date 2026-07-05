use once_cell::sync::Lazy;

use std::fs;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SecretsConfig {
    pub admin: SecretsConfigAdmin,
    pub contact: SecretsConfigContact,
    pub ipfs: SecretsConfigIpfs,
    pub website: SecretsConfigWebsite,
}

#[derive(Debug, Deserialize)]
pub struct SecretsConfigAdmin {
    pub public_key: String,
}

#[derive(Debug, Deserialize)]
pub struct SecretsConfigContact {
    pub arbitration_opt_out_email: String,
    pub dcma_email: String,
}

#[derive(Debug, Deserialize)]
pub struct SecretsConfigIpfs {
    pub protocol: String,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct SecretsConfigWebsite {
    pub hostname: String,
    pub port: u16,
}

static SECRETS_CONFIG: Lazy<SecretsConfig> = Lazy::new(|| {
    let secrets_toml = fs::read_to_string(format!(
        "{}/config/secrets.toml",
        env!("CARGO_MANIFEST_DIR")
    )).expect("Failed to read secrets.toml file.");
    toml::from_str(&secrets_toml).expect("config.toml is invalid")
});

pub fn secrets_config() -> &'static SecretsConfig {
    &SECRETS_CONFIG
}
