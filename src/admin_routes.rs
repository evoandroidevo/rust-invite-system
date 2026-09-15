use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, app_context},
    cookie::{Cookie, Cookies, SameSite, cookies},
    router::{
        HeaderValue, StatusCode,
        content::Form,
        error::{RouterErrorExt, SeeOther, see_other},
        header, page,
        request::headers,
        route,
    },
    view::{View, view},
};

use crate::admin_auth::{LoginError, SESSION_SECONDS, random_token, tokens_match};
use crate::app_state::AppState;

const SESSION_COOKIE: &str = "__Host-admin-session";
const LOGIN_COOKIE: &str = "__Host-admin-login";

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
    #[serde(default)]
    csrf: String,
}

#[derive(Deserialize)]
struct LogoutForm {
    #[serde(default)]
    csrf: String,
}

fn set_cookie(cx: &Cx, name: &'static str, value: String, seconds: i64) {
    cookies(cx).add(
        Cookie::build((name, value))
            .path("/")
            .secure(true)
            .http_only(true)
            .same_site(SameSite::Strict)
            .max_age(topcoat::cookie::time::Duration::seconds(seconds))
            .build(),
    );
}

pub fn require_admin(cx: &Cx) -> Result<String> {
    let state: &AppState = app_context(cx);
    let token = cookies(cx).get(SESSION_COOKIE).ok_or_unauthorized()?;
    Ok(state.admin.csrf_token(token.value()).ok_or_unauthorized()?)
}

fn require_origin(cx: &Cx) -> Result<()> {
    let state: &AppState = app_context(cx);
    let origin = headers(cx)
        .get("origin")
        .and_then(|value| value.to_str().ok());
    let fetch_site = headers(cx)
        .get("sec-fetch-site")
        .and_then(|value| value.to_str().ok());
    Ok((origin == Some(state.config.admin.origin.as_str())
        && !state.config.admin.origin.is_empty()
        && fetch_site.is_none_or(|site| site == "same-origin" || site == "none"))
    .then_some(())
    .ok_or_forbidden()?)
}

pub fn require_admin_mutation(cx: &Cx, csrf: &str) -> Result<()> {
    let expected = require_admin(cx)?;
    require_origin(cx)?;
    Ok(tokens_match(&expected, csrf)
        .then_some(())
        .ok_or_forbidden()?)
}

fn login_view(cx: &Cx, status: StatusCode, message: &'static str) -> Result<impl View> {
    let csrf = random_token();
    if status != StatusCode::SEE_OTHER {
        set_cookie(cx, LOGIN_COOKIE, csrf.clone(), 600);
    }
    Ok(view! {
        cx =>
        (status)
        ((header::CACHE_CONTROL, HeaderValue::from_static("no-store")))
        ((header::REFERRER_POLICY, HeaderValue::from_static("no-referrer")))
        if status == StatusCode::SEE_OTHER {
            ((header::LOCATION, HeaderValue::from_static("/admin")))
        }
        if status == StatusCode::TOO_MANY_REQUESTS {
            ((header::RETRY_AFTER, HeaderValue::from_static("60")))
        }
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <title>"Admin sign in | Rust Invite System"</title>
                <style>(crate::routes::ADMIN_CSS)</style>
            </head>
            <body>
                <main class="admin-shell">
                    <header class="admin-header">
                        <p class="eyebrow">"Rust Invite System"</p>
                        <h1>"Admin sign in"</h1>
                    </header>
                    <form action="/admin/login" method="post" style="max-width: 28rem; display: grid; gap: 1rem">
                        <input type="hidden" name="csrf" value=(csrf) />
                        <label>"Username" <input name="username" autocomplete="username" value="admin" required=(true) maxlength="64" /></label>
                        <label>"Password" <input name="password" type="password" autocomplete="current-password" required=(true) maxlength="1024" /></label>
                        if !message.is_empty() {
                            <p role="alert">(message)</p>
                        }
                        <button type="submit">"Sign in"</button>
                    </form>
                </main>
            </body>
        </html>
    })
}

#[page("/admin/login")]
async fn login_page(cx: &Cx) -> Result<impl View> {
    login_view(cx, StatusCode::OK, "")
}

#[page(POST "/admin/login")]
async fn login(cx: &Cx, Form(form): Form<LoginForm>) -> Result<impl View> {
    require_origin(cx)?;
    let challenge = cookies(cx).get(LOGIN_COOKIE).ok_or_forbidden()?;
    tokens_match(challenge.value(), &form.csrf)
        .then_some(())
        .ok_or_forbidden()?;
    let state: &AppState = app_context(cx);
    let outcome = state.admin.login(&form.username, form.password).await;
    let (status, message) = match outcome {
        Ok(token) => {
            set_cookie(cx, SESSION_COOKIE, token, SESSION_SECONDS as i64);
            set_cookie(cx, LOGIN_COOKIE, String::new(), 0);
            (StatusCode::SEE_OTHER, "")
        }
        Err(LoginError::InvalidCredentials) => (StatusCode::UNAUTHORIZED, "Sign in failed."),
        Err(LoginError::Throttled) => (
            StatusCode::TOO_MANY_REQUESTS,
            "Too many attempts. Try again in one minute.",
        ),
        Err(LoginError::Unavailable) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Sign in is temporarily unavailable.",
        ),
    };
    login_view(cx, status, message)
}

#[route(POST "/admin/logout")]
async fn logout(cx: &Cx, Form(form): Form<LogoutForm>) -> Result<SeeOther> {
    require_admin_mutation(cx, &form.csrf)?;
    let state: &AppState = app_context(cx);
    let token = cookies(cx).get(SESSION_COOKIE).ok_or_unauthorized()?;
    state
        .admin
        .logout(token.value())
        .map_err(|_| std::io::Error::other("Session invalidation failed"))?;
    set_cookie(cx, SESSION_COOKIE, String::new(), 0);
    Ok(see_other("/admin/login"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{admin_auth::hash_password, configuration::AppConfig};
    use topcoat::{
        asset::{AssetBundle, AssetConfig, RouterBuilderAssetExt},
        cookie::RouterBuilderCookieExt,
        router::{Body, Router, RouterBuilderDiscoverExt, request::Request, response::Response},
    };

    const PASSWORD: &str = "test-only admin password";
    const ORIGIN: &str = "https://invite.example.com";

    fn test_assets() -> AssetConfig {
        static ASSETS: std::sync::OnceLock<AssetConfig> = std::sync::OnceLock::new();
        ASSETS
            .get_or_init(|| {
                let directory = std::env::temp_dir()
                    .join(format!("invite-admin-test-{}", uuid::Uuid::new_v4()));
                let executable = std::fs::read(std::env::current_exe().unwrap()).unwrap();
                let config = topcoat_asset::BundlerConfig::new().cache_dir(directory.join("cache"));
                topcoat_asset::Bundler::new(&config)
                    .bundle(&executable, &directory)
                    .unwrap();
                let assets =
                    AssetConfig::hosted_at("/assets", AssetBundle::load_dir(&directory).unwrap());
                std::fs::remove_dir_all(directory).unwrap();
                assets
            })
            .clone()
    }

    async fn setup() -> (Router, AppState) {
        let mut config = AppConfig::default();
        config.database.url = "sqlite::memory:".into();
        config.admin.password_hash = hash_password(PASSWORD).unwrap();
        config.admin.origin = ORIGIN.into();
        let state = AppState::initialize(config).await.unwrap();
        let router = Router::builder()
            .discover()
            .cookies()
            .assets(test_assets())
            .app_context(state.clone())
            .build();
        (router, state)
    }

    fn request(
        method: &str,
        path: &str,
        cookie: &str,
        origin: &str,
        fields: &[(&str, &str)],
    ) -> Request {
        let mut encoded = reqwest::Url::parse("https://example.invalid").unwrap();
        encoded
            .query_pairs_mut()
            .extend_pairs(fields.iter().copied());
        Request::builder()
            .method(method)
            .uri(path)
            .header(header::HOST, "invite.example.com")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .header(header::COOKIE, cookie)
            .header(header::ORIGIN, origin)
            .body(Body::from(encoded.query().unwrap_or_default().to_owned()))
            .unwrap()
    }

    fn response_cookie(response: &Response, name: &str) -> Cookie<'static> {
        response
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|value| Cookie::parse(value.to_str().ok()?.to_owned()).ok())
            .find(|cookie| cookie.name() == name)
            .unwrap()
    }

    fn cookie_header(cookie: &Cookie<'_>) -> String {
        format!("{}={}", cookie.name(), cookie.value())
    }

    async fn sign_in(router: &Router) -> Cookie<'static> {
        let response = router
            .handle(request("GET", "/admin/login", "", ORIGIN, &[]))
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert_eq!(response.headers()[header::REFERRER_POLICY], "no-referrer");
        let challenge = response_cookie(&response, LOGIN_COOKIE);
        let response = router
            .handle(request(
                "POST",
                "/admin/login",
                &cookie_header(&challenge),
                ORIGIN,
                &[
                    ("username", "admin"),
                    ("password", PASSWORD),
                    ("csrf", challenge.value()),
                ],
            ))
            .await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(response.headers()[header::LOCATION], "/admin");
        let session = response_cookie(&response, SESSION_COOKIE);
        assert_eq!(session.secure(), Some(true));
        assert_eq!(session.http_only(), Some(true));
        assert_eq!(session.same_site(), Some(SameSite::Strict));
        assert_eq!(session.path(), Some("/"));
        assert!(session.domain().is_none());
        assert_eq!(
            session.max_age().unwrap().whole_seconds(),
            SESSION_SECONDS as i64
        );
        session
    }

    #[tokio::test]
    async fn unauthenticated_requests_and_proxy_headers_cannot_authorize_admin() {
        let (router, state) = setup().await;
        for (method, path, fields) in [
            ("GET", "/admin", vec![]),
            (
                "POST",
                "/admin/invites/generate",
                vec![("groups", "engineering"), ("expiration_hours", "1")],
            ),
            ("POST", "/admin/invites/revoke", vec![("id", "missing")]),
            ("POST", "/admin/logout", vec![]),
        ] {
            let mut incoming = request(method, path, "", ORIGIN, &fields);
            incoming
                .headers_mut()
                .insert("remote-user", HeaderValue::from_static("admin"));
            incoming
                .headers_mut()
                .insert("x-forwarded-for", HeaderValue::from_static("127.0.0.1"));
            assert_eq!(
                router.handle(incoming).await.status(),
                StatusCode::UNAUTHORIZED,
                "{path}"
            );
        }
        assert!(state.invites.list_recent(20).await.unwrap().is_empty());
        let forged = format!("{SESSION_COOKIE}={}", random_token());
        assert_eq!(
            router
                .handle(request("GET", "/admin", &forged, ORIGIN, &[]))
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn login_rejects_cross_origin_and_missing_csrf() {
        let (router, _) = setup().await;
        let response = router
            .handle(request("GET", "/admin/login", "", ORIGIN, &[]))
            .await;
        let challenge = response_cookie(&response, LOGIN_COOKIE);
        for (origin, csrf) in [
            ("https://attacker.example", challenge.value()),
            (ORIGIN, ""),
            ("", challenge.value()),
        ] {
            let response = router
                .handle(request(
                    "POST",
                    "/admin/login",
                    &cookie_header(&challenge),
                    origin,
                    &[
                        ("username", "admin"),
                        ("password", PASSWORD),
                        ("csrf", csrf),
                    ],
                ))
                .await;
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
            assert!(!response.headers().contains_key(header::SET_COOKIE));
        }
    }

    #[tokio::test]
    async fn authenticated_mutations_require_csrf_and_logout_invalidates_session() {
        let (router, state) = setup().await;
        let session = sign_in(&router).await;
        let cookie = cookie_header(&session);
        let csrf = state.admin.csrf_token(session.value()).unwrap();
        let dashboard = router
            .handle(request("GET", "/admin", &cookie, ORIGIN, &[]))
            .await;
        assert_eq!(dashboard.status(), StatusCode::OK);
        assert_eq!(dashboard.headers()[header::CACHE_CONTROL], "no-store");
        for (origin, token) in [
            (ORIGIN, ""),
            (ORIGIN, "invalid"),
            ("https://attacker.example", csrf.as_str()),
        ] {
            for (path, mut fields) in [
                (
                    "/admin/invites/generate",
                    vec![("groups", "engineering"), ("expiration_hours", "1")],
                ),
                ("/admin/invites/revoke", vec![("id", "missing")]),
                ("/admin/logout", vec![]),
            ] {
                fields.push(("csrf", token));
                assert_eq!(
                    router
                        .handle(request("POST", path, &cookie, origin, &fields))
                        .await
                        .status(),
                    StatusCode::FORBIDDEN,
                    "{path}"
                );
            }
        }
        assert!(state.invites.list_recent(20).await.unwrap().is_empty());
        let response = router
            .handle(request(
                "POST",
                "/admin/invites/generate",
                &cookie,
                ORIGIN,
                &[
                    ("groups", "engineering"),
                    ("expiration_hours", "1"),
                    ("csrf", &csrf),
                ],
            ))
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let records = state.invites.list_recent(20).await.unwrap();
        assert_eq!(records.len(), 1);
        let response = router
            .handle(request(
                "POST",
                "/admin/invites/revoke",
                &cookie,
                ORIGIN,
                &[("id", &records[0].id), ("csrf", &csrf)],
            ))
            .await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert!(
            state.invites.list_recent(20).await.unwrap()[0]
                .revoked_at
                .is_some()
        );
        let response = router
            .handle(request(
                "POST",
                "/admin/logout",
                &cookie,
                ORIGIN,
                &[("csrf", &csrf)],
            ))
            .await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response_cookie(&response, SESSION_COOKIE)
                .max_age()
                .unwrap()
                .whole_seconds(),
            0
        );
        assert_eq!(
            router
                .handle(request("GET", "/admin", &cookie, ORIGIN, &[]))
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn login_throttle_returns_retry_after() {
        let (router, _) = setup().await;
        let response = router
            .handle(request("GET", "/admin/login", "", ORIGIN, &[]))
            .await;
        let challenge = response_cookie(&response, LOGIN_COOKIE);
        for attempt in 0..6 {
            let response = router
                .handle(request(
                    "POST",
                    "/admin/login",
                    &cookie_header(&challenge),
                    ORIGIN,
                    &[
                        ("username", "admin"),
                        ("password", "wrong"),
                        ("csrf", challenge.value()),
                    ],
                ))
                .await;
            if attempt < 5 {
                assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            } else {
                assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
                assert_eq!(response.headers()[header::RETRY_AFTER], "60");
            }
        }
    }
}
