use std::str::FromStr;

use crate::admin_auth::AdminAuth;
use crate::configuration::AppConfig;
use crate::invite_storage::InviteRepository;
use crate::lldap::LldapClient;

#[derive(Clone, Debug)]
pub struct AppState {
    pub admin: AdminAuth,
    pub config: AppConfig,
    pub invites: InviteRepository,
    pub lldap: LldapClient,
}

impl AppState {
    pub async fn initialize(config: AppConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let admin = AdminAuth::new(config.admin.password_hash.clone())?;
        if !config.admin.password_hash.is_empty() {
            let origin = reqwest::Url::parse(&config.admin.origin)
                .map_err(|_| "Admin origin must be an HTTPS origin")?;
            if origin.scheme() != "https"
                || origin.origin().ascii_serialization() != config.admin.origin
            {
                return Err("Admin origin must be an HTTPS origin without a path".into());
            }
        }
        let options = sqlx::sqlite::SqliteConnectOptions::from_str(&config.database.url)?
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        let invites = InviteRepository::new(pool);
        let lldap = LldapClient::new(config.ldap.clone());
        invites.migrate().await?;

        Ok(Self {
            admin,
            config,
            invites,
            lldap,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn admin_startup_rejects_invalid_origin_and_hash() {
        let password_hash = crate::admin_auth::hash_password("test-only admin password").unwrap();
        for origin in [
            "",
            "http://invite.example.com",
            "https://invite.example.com/",
            "https://user:password@invite.example.com",
            "https://invite.example.com/path",
        ] {
            let mut config = AppConfig::default();
            config.database.url = "sqlite::memory:".into();
            config.admin.password_hash = password_hash.clone();
            config.admin.origin = origin.into();
            assert!(AppState::initialize(config).await.is_err());
        }
        let mut config = AppConfig::default();
        config.database.url = "sqlite::memory:".into();
        config.admin.password_hash = "invalid-hash".into();
        config.admin.origin = "https://invite.example.com".into();
        assert!(AppState::initialize(config).await.is_err());
    }
}
