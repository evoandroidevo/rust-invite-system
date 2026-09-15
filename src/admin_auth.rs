use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use rand::{Rng, distr::Alphanumeric};
use subtle::ConstantTimeEq;
use tokio::sync::Semaphore;

use crate::invite_storage::hash_token;

pub const SESSION_SECONDS: u64 = 8 * 60 * 60;
const LOGIN_WINDOW: Duration = Duration::from_secs(60);
const MAX_ATTEMPTS: u32 = 5;

#[derive(Clone)]
pub struct AdminAuth {
    username: String,
    password_hash: Arc<String>,
    state: Arc<Mutex<AuthState>>,
    verifier: Arc<Semaphore>,
}

struct AuthState {
    session: Option<Session>,
    window_started: Instant,
    attempts: u32,
}

struct Session {
    token_hash: String,
    csrf: String,
    expires: Instant,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LoginError {
    InvalidCredentials,
    Throttled,
    Unavailable,
}

impl std::fmt::Debug for AdminAuth {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("AdminAuth").finish_non_exhaustive()
    }
}

pub fn random_token() -> String {
    rand::rng()
        .sample_iter(Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}

pub fn tokens_match(expected: &str, supplied: &str) -> bool {
    expected.len() == 64
        && supplied.len() == 64
        && bool::from(expected.as_bytes().ct_eq(supplied.as_bytes()))
}

pub fn hash_password(password: &str) -> Result<String, &'static str> {
    if !(16..=1024).contains(&password.len()) {
        return Err("Admin password must contain 16 to 1024 bytes");
    }
    let salt_bytes: [u8; 16] = rand::random();
    let salt = SaltString::encode_b64(&salt_bytes).map_err(|_| "Salt generation failed")?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| "Password hashing failed")
}

impl AdminAuth {
    pub fn new(password_hash: String) -> Result<Self, &'static str> {
        Self::with_username("admin".to_owned(), password_hash)
    }

    pub fn with_username(username: String, password_hash: String) -> Result<Self, &'static str> {
        if username.is_empty()
            || username.len() > 64
            || username.trim() != username
            || username.chars().any(char::is_control)
        {
            return Err(
                "Admin username must contain 1 to 64 bytes without surrounding whitespace or control characters",
            );
        }
        if !password_hash.is_empty() {
            let parsed = PasswordHash::new(&password_hash).map_err(|_| "Invalid admin hash")?;
            let memory = parsed.params.get_decimal("m").unwrap_or(0);
            let iterations = parsed.params.get_decimal("t").unwrap_or(0);
            let parallelism = parsed.params.get_decimal("p").unwrap_or(0);
            if parsed.algorithm.as_str() != "argon2id"
                || parsed.version != Some(19)
                || !(19_456..=65_536).contains(&memory)
                || !(2..=5).contains(&iterations)
                || !(1..=4).contains(&parallelism)
                || parsed.salt.is_none_or(|salt| salt.len() < 22)
                || parsed.hash.is_none_or(|hash| hash.len() != 32)
            {
                return Err("Admin hash must use bounded Argon2id v19 parameters");
            }
        }
        Ok(Self {
            username,
            password_hash: Arc::new(password_hash),
            state: Arc::new(Mutex::new(AuthState {
                session: None,
                window_started: Instant::now(),
                attempts: 0,
            })),
            verifier: Arc::new(Semaphore::new(1)),
        })
    }

    fn reserve_attempt(&self, now: Instant) -> Result<(), LoginError> {
        let mut state = self.state.lock().map_err(|_| LoginError::Unavailable)?;
        if now.duration_since(state.window_started) >= LOGIN_WINDOW {
            state.window_started = now;
            state.attempts = 0;
        }
        if state.attempts >= MAX_ATTEMPTS {
            return Err(LoginError::Throttled);
        }
        state.attempts += 1;
        Ok(())
    }

    pub async fn login(&self, username: &str, password: String) -> Result<String, LoginError> {
        self.reserve_attempt(Instant::now())?;
        if self.password_hash.is_empty() || password.len() > 1024 {
            return Err(LoginError::InvalidCredentials);
        }
        let permit = self
            .verifier
            .clone()
            .try_acquire_owned()
            .map_err(|_| LoginError::Throttled)?;
        let stored_hash = self.password_hash.clone();
        let valid_username = username == self.username;
        let valid = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            PasswordHash::new(&stored_hash).is_ok_and(|parsed| {
                Argon2::default()
                    .verify_password(password.as_bytes(), &parsed)
                    .is_ok()
            })
        })
        .await
        .map_err(|_| LoginError::Unavailable)?;
        if !valid || !valid_username {
            return Err(LoginError::InvalidCredentials);
        }
        self.issue_session(Instant::now())
    }

    fn issue_session(&self, now: Instant) -> Result<String, LoginError> {
        let token = random_token();
        let mut state = self.state.lock().map_err(|_| LoginError::Unavailable)?;
        state.session = Some(Session {
            token_hash: hash_token(&token),
            csrf: random_token(),
            expires: now + Duration::from_secs(SESSION_SECONDS),
        });
        Ok(token)
    }

    pub fn csrf_token(&self, token: &str) -> Option<String> {
        self.csrf_token_at(token, Instant::now())
    }

    fn csrf_token_at(&self, token: &str, now: Instant) -> Option<String> {
        if token.len() != 64 {
            return None;
        }
        let mut state = self.state.lock().ok()?;
        if state
            .session
            .as_ref()
            .is_some_and(|session| now >= session.expires)
        {
            state.session = None;
        }
        let session = state.session.as_ref()?;
        tokens_match(&session.token_hash, &hash_token(token)).then(|| session.csrf.clone())
    }

    pub fn logout(&self, token: &str) -> Result<(), LoginError> {
        let mut state = self.state.lock().map_err(|_| LoginError::Unavailable)?;
        if state
            .session
            .as_ref()
            .is_some_and(|session| tokens_match(&session.token_hash, &hash_token(token)))
        {
            state.session = None;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn configured_username_replaces_default_and_matches_exactly() {
        let password = "test-only admin password";
        let auth =
            AdminAuth::with_username("operator".into(), hash_password(password).unwrap()).unwrap();
        for username in ["admin", "Operator", " operator"] {
            assert_eq!(
                auth.login(username, password.into()).await,
                Err(LoginError::InvalidCredentials)
            );
        }
        assert!(auth.login("operator", password.into()).await.is_ok());
    }

    #[test]
    fn rejects_invalid_configured_username() {
        for username in [
            String::new(),
            " ".into(),
            " operator".into(),
            "operator\n".into(),
            "x".repeat(65),
        ] {
            assert!(AdminAuth::with_username(username, String::new()).is_err());
        }
        assert!(AdminAuth::with_username("x".repeat(64), String::new()).is_ok());
    }

    #[tokio::test]
    async fn missing_credentials_disable_login() {
        let auth = AdminAuth::new(String::new()).unwrap();
        assert_eq!(
            auth.login("admin", "anything".into()).await,
            Err(LoginError::InvalidCredentials)
        );
    }

    #[tokio::test]
    async fn verifies_only_the_single_admin_and_preserves_password_bytes() {
        let password = " a sufficiently long password ";
        let auth = AdminAuth::new(hash_password(password).unwrap()).unwrap();
        assert_eq!(
            auth.login("other", password.into()).await,
            Err(LoginError::InvalidCredentials)
        );
        assert_eq!(
            auth.login("admin", password.trim().into()).await,
            Err(LoginError::InvalidCredentials)
        );
        let token = auth.login("admin", password.into()).await.unwrap();
        assert!(auth.csrf_token(&token).is_some());
        assert!(!format!("{auth:?}").contains("argon2"));
    }

    #[test]
    fn rejects_malformed_weak_and_excessive_hashes() {
        assert!(AdminAuth::new("not-a-hash".into()).is_err());
        let valid = hash_password("a sufficiently long password").unwrap();
        assert!(AdminAuth::new(valid.replace("m=19456", "m=8")).is_err());
        assert!(AdminAuth::new(valid.replace("m=19456", "m=9999999")).is_err());
        assert!(AdminAuth::new(valid.replace("argon2id", "argon2i")).is_err());
    }

    #[test]
    fn throttle_resets_only_after_window() {
        let auth = AdminAuth::new(String::new()).unwrap();
        let now = Instant::now();
        for _attempt in 0..MAX_ATTEMPTS {
            assert_eq!(auth.reserve_attempt(now), Ok(()));
        }
        assert_eq!(auth.reserve_attempt(now), Err(LoginError::Throttled));
        assert_eq!(auth.reserve_attempt(now + LOGIN_WINDOW), Ok(()));
    }

    #[test]
    fn sessions_rotate_expire_and_logout() {
        let auth = AdminAuth::new(String::new()).unwrap();
        let now = Instant::now();
        let first = auth.issue_session(now).unwrap();
        let second = auth.issue_session(now).unwrap();
        assert!(auth.csrf_token(&first).is_none());
        assert!(auth.csrf_token(&random_token()).is_none());
        let csrf = auth.csrf_token(&second).unwrap();
        assert!(tokens_match(&csrf, &csrf));
        assert!(!tokens_match(&csrf, &second));
        assert!(
            auth.csrf_token_at(&second, now + Duration::from_secs(SESSION_SECONDS))
                .is_none()
        );
        let third = auth.issue_session(now).unwrap();
        auth.logout(&second).unwrap();
        assert!(auth.csrf_token(&third).is_some());
        auth.logout(&third).unwrap();
        assert!(auth.csrf_token(&third).is_none());
    }
}
