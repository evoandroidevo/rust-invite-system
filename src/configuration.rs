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
            .add_source(
                Environment::with_prefix("APP")
                    .separator("__")
                    .try_parsing(true),
            )
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
    pub use_tls: bool,
    pub tls_insecure_skip_verify: bool,
    pub tls_ca_file: Option<String>,
    pub base_dn: String,
    pub username: String,
    pub password: String,
}

impl Default for LldapConfig {
    fn default() -> Self {
        Self {
            http_url: "http://127.0.0.1:17170".to_owned(),
            ldap_url: "ldap://127.0.0.1:3890".to_owned(),
            use_tls: false,
            tls_insecure_skip_verify: false,
            tls_ca_file: None,
            base_dn: "dc=example,dc=com".to_owned(),
            username: "admin".to_owned(),
            password: "dev-only-password-change-me".to_owned(),
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
        ffi::OsString,
        fs,
        sync::{Mutex, OnceLock},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::AppConfig;

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    struct EnvGuard {
        server_port: Option<OsString>,
        ldap_http_url: Option<OsString>,
    }

    impl EnvGuard {
        fn capture() -> Self {
            Self {
                server_port: std::env::var_os("APP__SERVER__PORT"),
                ldap_http_url: std::env::var_os("APP__LDAP__HTTP_URL"),
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            unsafe {
                match self.server_port.take() {
                    Some(value) => std::env::set_var("APP__SERVER__PORT", value),
                    None => std::env::remove_var("APP__SERVER__PORT"),
                }
                match self.ldap_http_url.take() {
                    Some(value) => std::env::set_var("APP__LDAP__HTTP_URL", value),
                    None => std::env::remove_var("APP__LDAP__HTTP_URL"),
                }
            }
        }
    }

    #[test]
    fn loads_toml_values_over_defaults() {
        let _guard = ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock should be available");
        let _env = EnvGuard::capture();
        unsafe {
            std::env::remove_var("APP__SERVER__PORT");
            std::env::remove_var("APP__LDAP__HTTP_URL");
        }

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
        assert!(!config.ldap.use_tls);
        assert!(!config.ldap.tls_insecure_skip_verify);
        assert!(config.ldap.tls_ca_file.is_none());
        assert_eq!(config.ldap.username, "admin");
        assert_eq!(config.appearance.default_theme.css_class(), "theme-dark");

        fs::remove_file(path).expect("test config should be removable");
    }

    #[test]
    fn environment_values_override_file_values() {
        let _guard = ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock should be available");
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time must be after the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("rust-invite-config-env-{suffix}.toml"));
        fs::write(
            &path,
            "[server]\nhost = \"0.0.0.0\"\nport = 9090\n[ldap]\nhttp_url = \"http://file.example\"\nbase_dn = \"dc=file,dc=example\"\n",
        )
        .expect("test config should be writable");

        let _env = EnvGuard::capture();

        unsafe {
            std::env::set_var("APP__SERVER__PORT", "7777");
            std::env::set_var("APP__LDAP__HTTP_URL", "http://env.example");
        }

        let config = AppConfig::load(&path).expect("test config should parse");

        assert_eq!(config.server.port, 7777);
        assert_eq!(config.ldap.http_url, "http://env.example");

        fs::remove_file(path).expect("test config should be removable");
    }
}
