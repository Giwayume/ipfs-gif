use std::io;
use std::error::Error;
use std::time::{ SystemTime, UNIX_EPOCH };

use base64::{ engine::general_purpose, Engine as _ };
use ed25519_dalek::{ Signature, VerifyingKey, pkcs8::DecodePublicKey };
use sha2::{ Digest, Sha256 };

pub fn verify_ed25519_signature(
    public_key_b64: &str,
    message: &str,
    signature_b64: &str,
) -> Result<(), Box<dyn Error>> {
    let public_key_der = general_purpose::STANDARD.decode(public_key_b64)?;
    let signature_bytes = general_purpose::STANDARD.decode(signature_b64)?;

    let verifying_key = VerifyingKey::from_public_key_der(&public_key_der)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("invalid public key der: {:?}", e)))?;

    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("invalid signature bytes: {:?}", e)))?;

    let message_bytes = message.as_bytes();
    verifying_key.verify_strict(message_bytes, &signature)?;

    Ok(())
}

pub fn random_message_to_sign(
    public_key_b64: &str,
    valid_from_unix_ms: u64,
    valid_until_unix_ms: u64,
) -> String {
    assert!(valid_from_unix_ms < valid_until_unix_ms);

    let public_key = general_purpose::STANDARD.decode(public_key_b64)
        .unwrap_or_else(|_| Vec::new());
    
    let pk_hash: [u8; 32] = Sha256::digest(public_key).into();

    // Deterministic nonce = H(pk_hash || start || end)
    let mut nonce_hasher = Sha256::new();
    nonce_hasher.update(b"sigmsg:nonce:v1");
    nonce_hasher.update(pk_hash);
    nonce_hasher.update(&valid_from_unix_ms.to_be_bytes());
    nonce_hasher.update(&valid_until_unix_ms.to_be_bytes());
    let nonce: [u8; 32] = nonce_hasher.finalize().into();

    // Tag = H(version || start || end || pk_hash || nonce)
    let mut tag_hasher = Sha256::new();
    tag_hasher.update(b"sigmsg:v1");
    tag_hasher.update(&valid_from_unix_ms.to_be_bytes());
    tag_hasher.update(&valid_until_unix_ms.to_be_bytes());
    tag_hasher.update(pk_hash);
    tag_hasher.update(nonce);
    let tag: [u8; 32] = tag_hasher.finalize().into();

    // Message bytes
    let mut msg = Vec::with_capacity(b"sigmsg:v1".len() + 8 + 8 + 32 + 32 + 32);
    msg.extend_from_slice(b"sigmsg:v1");
    msg.extend_from_slice(&valid_from_unix_ms.to_be_bytes());
    msg.extend_from_slice(&valid_until_unix_ms.to_be_bytes());
    msg.extend_from_slice(&pk_hash);
    msg.extend_from_slice(&nonce);
    msg.extend_from_slice(&tag);

    base64::engine::general_purpose::STANDARD.encode(msg)
}

pub fn random_message_to_sign_now_window(
    public_key_b64: &str,
    block_secs: u64,
    window_secs: u64,
    offset: i64,
) -> String {
    assert!(block_secs > 0);
    assert!(window_secs > 0);

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis() as u64;

    let now_secs = now_ms / 1000;

    // Block boundaries in seconds
    let block_start_secs = ((now_secs / block_secs) * block_secs).saturating_add(
        u64::try_from(
            i64::try_from(block_secs).unwrap_or(0).saturating_mul(offset)
        ).unwrap_or(0)
    );
    let valid_from_secs = block_start_secs;
    let valid_until_secs = valid_from_secs + window_secs;

    let valid_from_ms = valid_from_secs.saturating_mul(1000);
    let valid_until_ms = valid_until_secs.saturating_mul(1000);

    random_message_to_sign(public_key_b64, valid_from_ms, valid_until_ms)
}