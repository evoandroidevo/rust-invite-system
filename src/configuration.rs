use std::path::Path;

use config::{Config, Environment, File, FileFormat};
use serde::Deserialize;

use crate::validation::PasswordPolicy;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub ldap: LldapConfig,
    #[serde(default)]
    pub invites: InviteConfig,
    #[serde(default)]
    pub appearance: AppearanceConfig,
    #[serde(default)]
    pub password_policy: PasswordPolicy,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct AppearanceConfig {
    pub default_theme: Theme,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            default_theme: Theme::Light,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    pub fn css_class(&self) -> &'static str {
        match self {
            Self::Light => "theme-light",
            Self::Dark => "theme-dark",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    pub url: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite://./data/invites.db".to_owned(),
        }
    }
}

impl AppConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, config::ConfigError> {
        let path = path.as_ref().to_string_lossy();

        Config::builder()
            .add_source(File::new(&path, FileFormat::Toml).required(false))
            .add_source(Environment::with_prefix("APP").separator("__"))
            .build()?
            .try_deserialize()
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_owned(),
            port: 8080,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct LldapConfig {
    pub http_url: String,
    pub ldap_url: String,
    pub base_dn: String,
}

impl Default for LldapConfig {
    fn default() -> Self {
        Self {
            http_url: "http://127.0.0.1:17170".to_owned(),
            ldap_url: "ldap://127.0.0.1:3890".to_owned(),
            base_dn: "dc=example,dc=com".to_owned(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct InviteConfig {
    pub expiration_hours: i64,
}

impl Default for InviteConfig {
    fn default() -> Self {
        Self {
            expiration_hours: 48,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::AppConfig;

    #[test]
    fn loads_toml_values_over_defaults() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time must be after the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("rust-invite-config-{suffix}.toml"));
        fs::write(
            &path,
            "[server]\nhost = \"0.0.0.0\"\nport = 9090\n[invites]\nexpiration_hours = 12\n[appearance]\ndefault_theme = \"dark\"\n",
        )
        .expect("test config should be writable");

        let config = AppConfig::load(&path).expect("test config should parse");

        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 9090);
        assert_eq!(config.invites.expiration_hours, 12);
        assert_eq!(config.ldap.ldap_url, "ldap://127.0.0.1:3890");
        assert_eq!(config.appearance.default_theme.css_class(), "theme-dark");

        fs::remove_file(path).expect("test config should be removable");
    }
}
