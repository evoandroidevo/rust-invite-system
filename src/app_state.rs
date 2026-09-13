use std::str::FromStr;

use crate::configuration::AppConfig;
use crate::invite_storage::InviteRepository;
use crate::lldap::LldapClient;

#[derive(Clone, Debug)]
pub struct AppState {
    pub config: AppConfig,
    pub invites: InviteRepository,
    pub lldap: LldapClient,
}

impl AppState {
    pub async fn initialize(config: AppConfig) -> Result<Self, Box<dyn std::error::Error>> {
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
            config,
            invites,
            lldap,
        })
    }
}
