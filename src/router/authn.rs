use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::sync::RwLock;
use std::sync::{
    Arc,
    atomic::{ AtomicU64, Ordering },
};
use axum_login::{ AuthUser, AuthnBackend, UserId };
use async_trait::async_trait;
use serde::Deserialize;
use tokio::sync::OnceCell;

use crate::util::crypto;
use crate::util::geolocation::{ self, Geolocation };
use crate::util::secrets::secrets_config;

#[derive(Clone, Debug, Default, PartialEq)]
pub enum UserAccountType {
    Admin,
    #[default]
    Moderator,
}

#[allow(unused)]
#[derive(Debug, Default, Clone)]
pub struct ModeratorSession {
    pub id: u64,
    pub public_key: String,
    pub ip_address: String,
    pub latitude: f64,
    pub longitude: f64,
    pub account_type: UserAccountType,
}
impl AuthUser for ModeratorSession {
    type Id = u64;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.public_key.as_bytes()
    }
}

pub static MODERATOR_SESSIONS: OnceCell<RwLock<HashMap<u64, ModeratorSession>>> = OnceCell::const_new();
static MODERATOR_SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

pub async fn init_moderator_sessions() {
    MODERATOR_SESSIONS
        .set(RwLock::new(HashMap::new()))
        .expect("Moderator sessions already initialized.");
}

#[derive(Clone, Deserialize)]
pub struct Credentials {
    pub public_key: String,
    pub signed_message: String,
    pub ip_address: String,
}

#[derive(Clone, Default)]
pub struct Backend {
}

impl AuthnBackend for Backend {
    type User = ModeratorSession;
    type Credentials = Credentials;
    type Error = std::convert::Infallible;

    async fn authenticate(
        &self,
        Credentials { public_key, signed_message, ip_address }: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        let user = match async {

            let admin_public_key = secrets_config().admin.public_key.as_str();

            let location = match geolocation::find(&ip_address).await {
                Ok(location) => location,
                Err(_) => Geolocation::default(),
            };

            let messages_to_sign = Vec::from([
                crypto::random_message_to_sign_now_window(&public_key, 900, 900, 0),
                crypto::random_message_to_sign_now_window(&public_key, 900, 900, 1)
            ]);
            let mut signature_verification: Option<()> = None;
            for message_to_sign in messages_to_sign {
                if let Ok(_) = crypto::verify_ed25519_signature(&public_key, &message_to_sign, &signed_message) {
                    signature_verification = Some(());
                    break;
                }
            }
            if signature_verification.is_none() {
                return Err::<Self::User, Box<dyn std::error::Error>>(
                     Box::new(io::Error::new(io::ErrorKind::Other, "Signature verification failed."))
                )
            }

            let account_type = if admin_public_key == public_key {
                UserAccountType::Admin
            } else {
                UserAccountType::Moderator
            };

            let moderator_session = Self::User {
                id: MODERATOR_SESSION_COUNTER.fetch_add(1, Ordering::Relaxed),
                public_key: public_key.clone(),
                ip_address,
                latitude: location.latitude,
                longitude: location.longitude,
                account_type,
            };

            Ok::<Self::User, Box<dyn Error>>(moderator_session)
        }.await {
            Ok(user) => Some(user),
            Err(_) => None,
        };

        Ok(user)
    }

    async fn get_user(
        &self,
        id: &UserId<Self>,
    ) -> Result<Option<Self::User>, Self::Error> {
        let moderator_sessions = MODERATOR_SESSIONS.get().expect("Moderator sessions not initialized.");
        let moderator_sessions_read = moderator_sessions.read().unwrap_or_else(|poisoned| poisoned.into_inner());
        Ok(
            moderator_sessions_read.get(&id).cloned()
        )
    }
}

pub type AuthSession = axum_login::AuthSession<Backend>;

pub fn update_moderator_session_ip(id: &u64, ip_address: &str) {
    let moderator_sessions = MODERATOR_SESSIONS.get().expect("Moderator sessions not initialized.");
    let mut moderator_sessions_write = moderator_sessions.write().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(moderator_session) = moderator_sessions_write.get_mut(id) {
        moderator_session.ip_address = String::from(ip_address);
    }
}
