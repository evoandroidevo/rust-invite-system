use chrono::{DateTime, Duration, Utc};
use rand::{Rng, distr::Alphanumeric};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::time::Instant;
use topcoat::{
    Result,
    asset::{Asset, asset},
    context::{Cx, app_context},
    router::request::{headers, method, uri},
    router::{
        content::Form,
        error::{SeeOther, see_other},
        page, query_params, route,
    },
    view::{View, view},
};
use uuid::Uuid;

use crate::admin_routes::{require_admin, require_admin_mutation};
use crate::app_state::AppState;
use crate::invite_storage::{InviteRecord, hash_token};
use crate::lldap::LldapError;
use crate::validation::{validate_email, validate_password, validate_username};
use crate::version::app_version;
use crate::views;

#[query_params(error = bad_request)]
struct InviteQuery {
    code: Option<String>,
}

#[query_params(error = bad_request)]
struct SubmitInviteQuery {
    code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GenerateInviteForm {
    #[serde(default)]
    csrf: String,
    groups: String,
    expiration_hours: i64,
}

#[derive(Debug, Deserialize)]
struct RevokeInviteForm {
    #[serde(default)]
    csrf: String,
    id: String,
}

fn invite_status(record: &InviteRecord, now: DateTime<Utc>) -> &'static str {
    if record.revoked_at.is_some() {
        "Revoked"
    } else if record.consumed_at.is_some() {
        "Used"
    } else {
        match record.expires_at.parse::<DateTime<Utc>>() {
            Ok(expires_at) if expires_at <= now => "Expired",
            Ok(_) => "Active",
            Err(_) => "Invalid",
        }
    }
}

fn invite_is_active(record: &InviteRecord, now: DateTime<Utc>) -> bool {
    record.revoked_at.is_none()
        && record.consumed_at.is_none()
        && record
            .expires_at
            .parse::<DateTime<Utc>>()
            .is_ok_and(|expires_at| expires_at > now)
}

fn header_value(cx: &Cx, name: &str) -> String {
    headers(cx)
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| String::from("-"))
}

fn request_client_ip(cx: &Cx) -> String {
    let forwarded_for = header_value(cx, "x-forwarded-for");
    let forwarded_ip = forwarded_for
        .split(',')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("-");

    if forwarded_ip != "-" {
        forwarded_ip.to_owned()
    } else {
        let real_ip = header_value(cx, "x-real-ip");
        if real_ip != "-" {
            real_ip
        } else {
            String::from("unknown")
        }
    }
}

struct RequestLog {
    method: String,
    path: String,
    client_ip: String,
    forwarded_for: String,
    user_agent: String,
    referer: String,
    message_field: String,
    started_at: Instant,
}

impl RequestLog {
    fn new(cx: &Cx, message_field: &str) -> Self {
        let uri = uri(cx);
        let request = Self {
            method: method(cx).to_string(),
            path: uri.path().to_owned(),
            client_ip: request_client_ip(cx),
            forwarded_for: header_value(cx, "x-forwarded-for"),
            user_agent: header_value(cx, "user-agent"),
            referer: header_value(cx, "referer"),
            message_field: message_field.to_owned(),
            started_at: Instant::now(),
        };

        request.emit_json("info", "request start", None);

        request
    }

    fn emit_json(&self, level: &str, message: &str, duration_ms: Option<u128>) {
        let mut fields = Map::new();
        fields.insert(
            String::from("ts"),
            Value::from(chrono::Utc::now().to_rfc3339()),
        );
        fields.insert(String::from("level"), Value::from(level));
        fields.insert(self.message_field.clone(), Value::from(message));
        fields.insert(String::from("method"), Value::from(self.method.clone()));
        fields.insert(String::from("path"), Value::from(self.path.clone()));
        fields.insert(
            String::from("client_ip"),
            Value::from(self.client_ip.clone()),
        );
        fields.insert(
            String::from("forwarded_for"),
            Value::from(self.forwarded_for.clone()),
        );
        fields.insert(
            String::from("user_agent"),
            Value::from(self.user_agent.clone()),
        );
        fields.insert(String::from("referer"), Value::from(self.referer.clone()));

        if let Some(duration_ms) = duration_ms {
            fields.insert(
                String::from("duration_ms"),
                Value::from(duration_ms.min(u128::from(u64::MAX)) as u64),
            );
        }

        eprintln!("{}", Value::Object(fields));
    }
}

impl Drop for RequestLog {
    fn drop(&mut self) {
        self.emit_json(
            "info",
            "request complete",
            Some(self.started_at.elapsed().as_millis()),
        );
    }
}

const INVITE_CSS: &str = r#"
    :root {
        color-scheme: light dark;
        --page-bg: #f5f1eb;
        --card-bg: rgba(255, 253, 249, .94);
        --text: #17212b;
        --muted: #59636c;
        --label: #263440;
        --input-bg: #fffdfa;
        --border: #c9c2ba;
        --button: #b24b35;
        font-family: "Avenir Next", "Segoe UI", sans-serif;
        color: var(--text);
        background: var(--page-bg);
    }
    * { box-sizing: border-box; }
    body {
        min-height: 100vh;
        margin: 0;
        background: radial-gradient(circle at top left, #f9d7c5 0, transparent 34rem), var(--page-bg);
        color: var(--text);
    }
    body.theme-dark {
        --page-bg: #17212b;
        --card-bg: rgba(29, 40, 50, .96);
        --text: #f4f0e9;
        --muted: #b8c0c5;
        --label: #e5e1da;
        --input-bg: #24313c;
        --border: #52606b;
        --button: #e07a5f;
        color-scheme: dark;
    }
    .signup-shell {
        display: grid;
        min-height: 100vh;
        place-items: center;
        padding: 2rem 1rem;
    }
    .signup-card {
        width: min(100%, 34rem);
        padding: clamp(1.5rem, 5vw, 3rem);
        border: 1px solid rgba(23, 33, 43, .12);
        border-radius: 1rem;
        background: var(--card-bg);
        box-shadow: 0 1.5rem 4rem rgba(67, 47, 37, .14);
    }
    .eyebrow {
        margin: 0 0 .75rem;
        color: #b24b35;
        font-size: .75rem;
        font-weight: 800;
        letter-spacing: .08em;
        text-transform: uppercase;
    }
    h1 {
        margin: 0;
        font-size: clamp(1.8rem, 5vw, 2.5rem);
        line-height: 1.05;
    }
    .intro {
        margin: .9rem 0 2rem;
        color: var(--muted);
        line-height: 1.55;
    }
    form { display: grid; gap: 1rem; }
    .field-row {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 1rem;
    }
    label {
        display: grid;
        gap: .45rem;
        color: var(--label);
        font-size: .88rem;
        font-weight: 700;
    }
    input {
        width: 100%;
        min-height: 2.9rem;
        padding: .7rem .8rem;
        border: 1px solid var(--border);
        border-radius: .55rem;
        background: var(--input-bg);
        color: var(--text);
        font: inherit;
        font-weight: 500;
    }
    input:focus {
        border-color: #b24b35;
        outline: 3px solid rgba(178, 75, 53, .18);
    }
    .password-note {
        margin: -.35rem 0 .25rem;
        color: #69737b;
        font-size: .78rem;
        font-weight: 500;
    }
    .form-errors {
        margin: 0 0 1rem;
        padding: .75rem 1rem .75rem 1.5rem;
        border: 1px solid #c0392b;
        border-radius: .55rem;
        background: rgba(192, 57, 43, .1);
        color: #c0392b;
        font-size: .85rem;
        line-height: 1.5;
    }
    body.theme-dark .form-errors {
        border-color: #e07a5f;
        background: rgba(224, 122, 95, .16);
        color: #f4b8a4;
    }
    button {
        min-height: 3rem;
        margin-top: .5rem;
        border: 0;
        border-radius: .55rem;
        background: var(--button);
        color: #fffdfa;
        cursor: pointer;
        font: inherit;
        font-weight: 800;
    }
    button:hover { filter: brightness(.92); }
    button:focus-visible { outline: 3px solid rgba(178, 75, 53, .3); outline-offset: 2px; }
    .card-header {
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: 1rem;
    }
    .theme-toggle {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        gap: .45rem;
        min-height: 2.25rem;
        margin: 0;
        padding: .45rem .7rem;
        border: 1px solid var(--border);
        background: transparent;
        color: var(--text);
        font-size: .78rem;
    }
    .theme-icon { width: 3.75rem; height: auto; display: block; }
    .theme-icon-dark { display: none; }
    body.theme-dark .theme-icon-light { display: none; }
    body.theme-dark .theme-icon-dark { display: block; }
    @media (max-width: 30rem) {
        .field-row { grid-template-columns: 1fr; }
    }
"#;

const THEME_LIGHT_ICON: Asset = asset!("../assets/icons/theme-light.svg");
const THEME_DARK_ICON: Asset = asset!("../assets/icons/theme-dark.svg");

const THEME_SCRIPT: &str = r#"
    const body = document.body;
    const toggle = document.getElementById('theme-toggle');
    const savedTheme = localStorage.getItem('invite-theme');
    if (savedTheme === 'light' || savedTheme === 'dark') {
        body.classList.remove('theme-light', 'theme-dark');
        body.classList.add(`theme-${savedTheme}`);
    }
    function updateThemeLabel() {
        const isDark = body.classList.contains('theme-dark');
        toggle.querySelector('.theme-label').textContent = isDark ? 'Use light mode' : 'Use dark mode';
        toggle.setAttribute('aria-pressed', String(isDark));
    }
    updateThemeLabel();
    toggle.addEventListener('click', function () {
        const nextTheme = body.classList.contains('theme-dark') ? 'light' : 'dark';
        body.classList.remove('theme-light', 'theme-dark');
        body.classList.add(`theme-${nextTheme}`);
        localStorage.setItem('invite-theme', nextTheme);
        updateThemeLabel();
    });
"#;

pub(crate) const ADMIN_CSS: &str = r#"
    :root {
        color-scheme: light dark;
        --page-bg: #f5f1eb;
        --card-bg: rgba(255, 253, 249, .96);
        --text: #17212b;
        --muted: #59636c;
        --label: #263440;
        --input-bg: #fffdfa;
        --border: #c9c2ba;
        --accent: #b24b35;
        font-family: "Avenir Next", "Segoe UI", sans-serif;
    }
    * { box-sizing: border-box; }
    body {
        min-height: 100vh;
        margin: 0;
        color: var(--text);
        background: radial-gradient(circle at top left, #f9d7c5 0, transparent 34rem), var(--page-bg);
    }
    body.theme-dark {
        --page-bg: #17212b;
        --card-bg: #1d2832;
        --text: #f4f0e9;
        --muted: #b8c0c5;
        --label: #e5e1da;
        --input-bg: #24313c;
        --border: #52606b;
        --accent: #e07a5f;
        color-scheme: dark;
    }
    .admin-shell {
        width: min(100%, 54rem);
        margin: 0 auto;
        padding: 3rem 1rem;
    }
    .admin-header { margin-bottom: 2rem; }
    .eyebrow {
        margin: 0 0 .65rem;
        color: var(--accent);
        font-size: .75rem;
        font-weight: 800;
        letter-spacing: .08em;
        text-transform: uppercase;
    }
    h1 { margin: 0; font-size: clamp(2rem, 5vw, 3rem); line-height: 1.05; }
    .intro { max-width: 38rem; margin: .9rem 0 0; color: var(--muted); line-height: 1.55; }
    .admin-grid { display: grid; grid-template-columns: minmax(0, 1.35fr) minmax(15rem, .65fr); gap: 1rem; }
    .admin-card {
        padding: clamp(1.25rem, 4vw, 2rem);
        border: 1px solid color-mix(in srgb, var(--text) 14%, transparent);
        border-radius: 1rem;
        background: var(--card-bg);
        box-shadow: 0 1.5rem 4rem rgba(67, 47, 37, .12);
    }
    .admin-card h2 { margin: 0 0 .5rem; font-size: 1.15rem; }
    .admin-card p { margin: 0 0 1.5rem; color: var(--muted); line-height: 1.5; }
    form { display: grid; gap: 1rem; }
    label { display: grid; gap: .45rem; color: var(--label); font-size: .88rem; font-weight: 700; }
    input {
        width: 100%;
        min-height: 2.9rem;
        padding: .7rem .8rem;
        border: 1px solid var(--border);
        border-radius: .55rem;
        background: var(--input-bg);
        color: var(--text);
        font: inherit;
    }
    input:focus { border-color: var(--accent); outline: 3px solid color-mix(in srgb, var(--accent) 22%, transparent); }
    button {
        min-height: 3rem;
        margin-top: .35rem;
        border: 0;
        border-radius: .55rem;
        background: var(--accent);
        color: #fffdfa;
        cursor: pointer;
        font: inherit;
        font-weight: 800;
    }
    button:hover { filter: brightness(.92); }
    .stat { display: grid; gap: .35rem; padding-top: 1rem; border-top: 1px solid var(--border); }
    .stat strong { font-size: 1.7rem; }
    .stat span { color: var(--muted); font-size: .82rem; }
    .theme-toggle {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        gap: .45rem;
        min-height: 2.25rem;
        margin: 0;
        padding: .45rem .7rem;
        border: 1px solid var(--border);
        background: transparent;
        color: var(--text);
        font-size: .78rem;
    }
    .theme-icon { width: 3.75rem; height: auto; display: block; }
    .theme-icon-dark { display: none; }
    body.theme-dark .theme-icon-light { display: none; }
    body.theme-dark .theme-icon-dark { display: block; }
    .admin-header-row {
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: 1rem;
    }
    .history-card { margin-top: 1rem; }
    .history-table {
        width: 100%;
        border-collapse: collapse;
        font-size: .88rem;
    }
    .history-table th, .history-table td {
        padding: .6rem .5rem;
        border-bottom: 1px solid var(--border);
        text-align: left;
        vertical-align: top;
    }
    .history-table th { color: var(--muted); font-weight: 700; font-size: .78rem; text-transform: uppercase; letter-spacing: .04em; }
    .hash-cell { font-family: ui-monospace, monospace; font-size: .74rem; cursor: help; }
    .status-badge {
        display: inline-block;
        padding: .2rem .55rem;
        border-radius: .4rem;
        font-size: .74rem;
        font-weight: 800;
        text-transform: uppercase;
        letter-spacing: .03em;
    }
    .status-active { background: color-mix(in srgb, #2f9e44 20%, transparent); color: #2f9e44; }
    .status-used { background: color-mix(in srgb, var(--muted) 20%, transparent); color: var(--muted); }
    .status-expired { background: color-mix(in srgb, #c9820a 20%, transparent); color: #c9820a; }
    .status-revoked { background: color-mix(in srgb, var(--accent) 20%, transparent); color: var(--accent); }
    .revoke-form { margin: 0; }
    .revoke-button {
        min-height: 2rem;
        margin: 0;
        padding: .35rem .6rem;
        border: 1px solid var(--border);
        border-radius: .4rem;
        background: transparent;
        color: var(--accent);
        font-size: .78rem;
        font-weight: 700;
    }
    .revoke-button:hover { background: color-mix(in srgb, var(--accent) 12%, transparent); }
    dialog#invite-modal {
        width: min(92vw, 32rem);
        padding: 0;
        border: none;
        border-radius: 1rem;
        background: transparent;
        overflow: visible;
    }
    dialog#invite-modal::backdrop { background: rgba(23, 33, 43, .55); }
    dialog#invite-modal .admin-card { margin: 0; }
    dialog#invite-modal #invite-modal-close { margin-top: 1rem; }
    @media (max-width: 42rem) { .admin-grid { grid-template-columns: 1fr; } }
"#;

const ADMIN_MODAL_SCRIPT: &str = r#"
    const generateForm = document.getElementById('generate-invite-form');
    const modal = document.getElementById('invite-modal');
    const modalContent = document.getElementById('invite-modal-content');
    const modalClose = document.getElementById('invite-modal-close');

    generateForm.addEventListener('submit', function (event) {
        event.preventDefault();
        fetch(generateForm.action, {
            method: 'POST',
            body: new URLSearchParams(new FormData(generateForm)),
        })
            .then(function (response) { return response.text(); })
            .then(function (html) {
                modalContent.innerHTML = html;
                modal.showModal();
            })
            .catch(function () {
                modalContent.innerHTML = '<section class="admin-card"><h1>Unable to generate invitation</h1><p>A network error occurred. Try again.</p></section>';
                modal.showModal();
            });
    });

    modalClose.addEventListener('click', function () {
        modal.close();
    });

    modal.addEventListener('close', function () {
        window.location.reload();
    });
"#;

#[page("/")]
async fn home(cx: &Cx) -> Result<impl View> {
    let state: &AppState = app_context(cx);
    let _request_log = RequestLog::new(cx, &state.config.logging.message_field);

    Ok(view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <title>(views::page_title())</title>
            </head>
            <body>
                <h1>(views::page_title())</h1>
                <p>"Invite service is starting."</p>
                <p>"Version: " (app_version())</p>
            </body>
        </html>
    })
}

#[page("/health")]
async fn health(cx: &Cx) -> Result<impl View> {
    let state: &AppState = app_context(cx);
    let _request_log = RequestLog::new(cx, &state.config.logging.message_field);

    Ok(view! {
        <p>"ok (version: " (app_version()) ", invite expiry: " (state.config.invites.expiration_hours) " hours)"</p>
    })
}

#[page("/admin")]
async fn admin(cx: &Cx) -> Result<impl View> {
    let csrf = require_admin(cx)?;
    let state: &AppState = app_context(cx);
    let _request_log = RequestLog::new(cx, &state.config.logging.message_field);
    let theme_class = state.config.appearance.default_theme.css_class();
    let expiration_hours = state.config.invites.expiration_hours;
    let now = Utc::now();
    let invites = state.invites.list_recent(20).await.unwrap_or_default();

    Ok(view! {
        ((topcoat::router::header::CACHE_CONTROL, topcoat::router::HeaderValue::from_static("no-store")))
        ((topcoat::router::header::REFERRER_POLICY, topcoat::router::HeaderValue::from_static("no-referrer")))
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <title>"Admin | Rust Invite System"</title>
                <style>(ADMIN_CSS)</style>
            </head>
            <body class=(theme_class)>
                <main class="admin-shell">
                    <header class="admin-header">
                        <div class="admin-header-row">
                            <div>
                                <p class="eyebrow">"Rust Invite System / Admin"</p>
                                <h1>"Invite dashboard"</h1>
                            </div>
                            <button class="theme-toggle" id="theme-toggle" type="button" aria-label="Toggle color theme">
                                <img class="theme-icon theme-icon-light" src=(THEME_LIGHT_ICON) alt="" aria-hidden="true" />
                                <img class="theme-icon theme-icon-dark" src=(THEME_DARK_ICON) alt="" aria-hidden="true" />
                                <span class="theme-label">"Toggle theme"</span>
                            </button>
                        </div>
                        <form action="/admin/logout" method="post">
                            <input type="hidden" name="csrf" value=(csrf.clone()) />
                            <button type="submit">"Sign out"</button>
                        </form>
                        <p class="intro">"Create one-time invitation links and assign the directory groups new accounts should receive."</p>
                    </header>
                    <div class="admin-grid">
                        <section class="admin-card" aria-labelledby="generate-title">
                            <h2 id="generate-title">"Generate an invitation"</h2>
                            <p>"The generated link will be available after the invite service validates and stores these settings."</p>
                            <form id="generate-invite-form" action="/admin/invites/generate" method="post">
                                <input type="hidden" name="csrf" value=(csrf.clone()) />
                                <label>
                                    "Directory groups"
                                    <input type="text" name="groups" placeholder="developers, vpn-users" autocomplete="off" required=(true) />
                                </label>
                                <label>
                                    "Expires after"
                                    <input type="number" name="expiration_hours" min="1" value=(expiration_hours) required=(true) />
                                </label>
                                <button type="submit">"Generate invite link"</button>
                            </form>
                        </section>
                        <aside class="admin-card" aria-labelledby="status-title">
                            <h2 id="status-title">"Service status"</h2>
                            <p>"Invite storage is connected and ready for generation."</p>
                            <div class="stat">
                                <strong>(expiration_hours) "h"</strong>
                                <span>"Configured default expiration"</span>
                            </div>
                            <div class="stat">
                                <strong>"Protected"</strong>
                                <span>"Signed in as admin"</span>
                            </div>
                        </aside>
                    </div>
                    <section class="admin-card history-card" aria-labelledby="history-title">
                        <h2 id="history-title">"Invite history"</h2>
                        <p>"The most recent invitations, newest first."</p>
                        if invites.is_empty() {
                            <p>"No invitations have been generated yet."</p>
                        } else {
                            <table class="history-table">
                                <thead>
                                    <tr>
                                        <th>"Groups"</th>
                                        <th>"Hash"</th>
                                        <th>"Created"</th>
                                        <th>"Expires"</th>
                                        <th>"Status"</th>
                                        <th>"Action"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    for record in &invites {
                                        <tr>
                                            <td>(serde_json::from_str::<Vec<String>>(&record.groups_json).unwrap_or_default().join(", "))</td>
                                            <td class="hash-cell" title=(record.token_hash.clone())>(record.token_hash.get(..12).unwrap_or(&record.token_hash)) "…"</td>
                                            <td>(record.created_at.clone())</td>
                                            <td>(record.expires_at.clone())</td>
                                            <td>
                                                if invite_status(record, now) == "Active" {
                                                    <span class="status-badge status-active">"Active"</span>
                                                } else if invite_status(record, now) == "Used" {
                                                    <span class="status-badge status-used">"Used"</span>
                                                } else if invite_status(record, now) == "Expired" {
                                                    <span class="status-badge status-expired">"Expired"</span>
                                                } else {
                                                    <span class="status-badge status-revoked">"Revoked"</span>
                                                }
                                            </td>
                                            <td>
                                                if invite_status(record, now) == "Active" {
                                                    <form class="revoke-form" action="/admin/invites/revoke" method="post">
                                                        <input type="hidden" name="csrf" value=(csrf.clone()) />
                                                        <input type="hidden" name="id" value=(record.id.clone()) />
                                                        <button class="revoke-button" type="submit">"Disable"</button>
                                                    </form>
                                                } else {
                                                    "—"
                                                }
                                            </td>
                                        </tr>
                                    }
                                </tbody>
                            </table>
                        }
                    </section>
                </main>
                <dialog id="invite-modal">
                    <div id="invite-modal-content"></div>
                    <button class="revoke-button" type="button" id="invite-modal-close">"Close"</button>
                </dialog>
                <script>(THEME_SCRIPT)</script>
                <script>(ADMIN_MODAL_SCRIPT)</script>
            </body>
        </html>
    })
}

#[route(POST "/admin/invites/revoke")]
async fn revoke_invite_route(cx: &Cx, Form(form): Form<RevokeInviteForm>) -> Result<SeeOther> {
    require_admin_mutation(cx, &form.csrf)?;
    let state: &AppState = app_context(cx);
    let _request_log = RequestLog::new(cx, &state.config.logging.message_field);
    let _ = state
        .invites
        .revoke_invite(&form.id, &Utc::now().to_rfc3339())
        .await;

    Ok(see_other("/admin"))
}

#[page(POST "/admin/invites/generate")]
async fn generate_invite(cx: &Cx, Form(form): Form<GenerateInviteForm>) -> Result<impl View> {
    require_admin_mutation(cx, &form.csrf)?;
    let state: &AppState = app_context(cx);
    let _request_log = RequestLog::new(cx, &state.config.logging.message_field);
    let groups: Vec<String> = form
        .groups
        .split(',')
        .map(str::trim)
        .filter(|group| !group.is_empty())
        .map(ToOwned::to_owned)
        .collect();

    let (message, invite_url, groups_text) = if groups.is_empty() {
        (
            String::from("Enter at least one directory group."),
            None,
            None,
        )
    } else if form.expiration_hours < 1 {
        (
            String::from("Expiration must be at least one hour."),
            None,
            None,
        )
    } else {
        let rng = rand::rng();
        let token: String = rng
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        let now = Utc::now();
        let expires_at = now + Duration::hours(form.expiration_hours);
        let repository = &state.invites;
        let invite_url = format!("/invite?code={token}");

        if repository
            .create_invite(
                &Uuid::new_v4().to_string(),
                &hash_token(&token),
                &serde_json::to_string(&groups).expect("groups should serialize"),
                &now.to_rfc3339(),
                &expires_at.to_rfc3339(),
                None,
            )
            .await
            .is_err()
        {
            (
                String::from("The invitation could not be stored. Try again."),
                None,
                None,
            )
        } else {
            (
                format!(
                    "This link expires in {} hours and can be used once.",
                    form.expiration_hours
                ),
                Some(invite_url),
                Some(groups.join(", ")),
            )
        }
    };

    Ok(view! {
        <section class="admin-card">
            ((topcoat::router::header::CACHE_CONTROL, topcoat::router::HeaderValue::from_static("no-store")))
            ((topcoat::router::header::REFERRER_POLICY, topcoat::router::HeaderValue::from_static("no-referrer")))
            <p class="eyebrow">"Invitation ready"</p>
            <h1>"Share this invite link"</h1>
            <p>(message)</p>
            if let Some(invite_url) = invite_url {
                <label>
                    "Invite URL"
                    <input type="text" readonly=(true) value=(invite_url) />
                </label>
            }
            if let Some(groups) = groups_text {
                <p>"Groups: " (groups)</p>
            }
        </section>
    })
}

#[derive(Debug, Deserialize)]
struct SubmitInviteForm {
    username: String,
    email: String,
    first_name: String,
    last_name: String,
    password: String,
}

#[allow(clippy::too_many_arguments)]
fn invite_form_view(
    cx: &Cx,
    theme_class: String,
    code: String,
    username: String,
    email: String,
    first_name: String,
    last_name: String,
    errors: Vec<String>,
    success_message: Option<String>,
) -> Result<impl View> {
    Ok(view! {
        cx =>
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <title>"Accept invitation"</title>
                <style>(INVITE_CSS)</style>
            </head>
            <body class=(format!("signup-shell {theme_class}"))>
                <main class="signup-shell">
                    <section class="signup-card" aria-labelledby="signup-title">
                        <div class="card-header">
                            <div>
                                <p class="eyebrow">"Rust Invite System"</p>
                                <h1 id="signup-title">"Create your account"</h1>
                            </div>
                            <button class="theme-toggle" id="theme-toggle" type="button" aria-label="Toggle color theme">
                                <img class="theme-icon theme-icon-light" src=(THEME_LIGHT_ICON) alt="" aria-hidden="true" />
                                <img class="theme-icon theme-icon-dark" src=(THEME_DARK_ICON) alt="" aria-hidden="true" />
                                <span class="theme-label">"Toggle theme"</span>
                            </button>
                        </div>
                        <p class="intro">"Complete the form below to activate your account."</p>
                        if !errors.is_empty() {
                            <ul class="form-errors">
                                for message in &errors {
                                    <li>(message.clone())</li>
                                }
                            </ul>
                        }
                        if let Some(success_message) = success_message {
                            <p class="form-success">(success_message)</p>
                        }
                        <form action=(format!("/invite/submit?code={code}")) method="post">
                            <label>
                                "Username"
                                <input type="text" name="username" autocomplete="username" value=(username) required=(true) />
                            </label>
                            <label>
                                "Email"
                                <input type="email" name="email" autocomplete="email" value=(email) required=(true) />
                            </label>
                            <div class="field-row">
                                <label>
                                    "First name"
                                    <input type="text" name="first_name" autocomplete="given-name" value=(first_name) required=(true) />
                                </label>
                                <label>
                                    "Last name"
                                    <input type="text" name="last_name" autocomplete="family-name" value=(last_name) required=(true) />
                                </label>
                            </div>
                            <label>
                                "Password"
                                <input type="password" name="password" autocomplete="new-password" required=(true) />
                            </label>
                            <p class="password-note">"Use at least 12 characters with a mix of letters, numbers, and symbols."</p>
                            <button type="submit">"Create account"</button>
                        </form>
                    </section>
                </main>
                <script>(THEME_SCRIPT)</script>
            </body>
        </html>
    })
}

#[page("/invite")]
async fn invite(cx: &Cx) -> Result<impl View> {
    let state: &AppState = app_context(cx);
    let _request_log = RequestLog::new(cx, &state.config.logging.message_field);
    let query = query_params::<InviteQuery>(cx)?;
    let code = query.code.clone().unwrap_or_default();
    let theme_class = state.config.appearance.default_theme.css_class().to_owned();

    invite_form_view(
        cx,
        theme_class,
        code,
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        Vec::new(),
        None,
    )
}

#[page(POST "/invite/submit")]
async fn submit_invite(cx: &Cx, Form(form): Form<SubmitInviteForm>) -> Result<impl View> {
    let state: &AppState = app_context(cx);
    let _request_log = RequestLog::new(cx, &state.config.logging.message_field);
    let theme_class = state.config.appearance.default_theme.css_class().to_owned();
    let query = query_params::<SubmitInviteQuery>(cx)?;
    let code = match query.code.clone() {
        Some(code) if !code.trim().is_empty() => code,
        _ => {
            return invite_form_view(
                cx,
                theme_class,
                String::new(),
                form.username,
                form.email,
                form.first_name,
                form.last_name,
                vec![String::from("This invitation link is missing its code.")],
                None,
            );
        }
    };

    let mut errors = Vec::new();
    if let Err(message) = validate_username(&form.username) {
        errors.push(message);
    }
    if let Err(message) = validate_email(&form.email) {
        errors.push(message);
    }
    if let Err(message) = validate_password(&form.password, &state.config.password_policy) {
        errors.push(message);
    }

    if !errors.is_empty() {
        return invite_form_view(
            cx,
            theme_class,
            code,
            form.username,
            form.email,
            form.first_name,
            form.last_name,
            errors,
            None,
        );
    }

    if state
        .lldap
        .username_exists(&form.username)
        .await
        .unwrap_or(false)
    {
        return invite_form_view(
            cx,
            theme_class,
            code,
            form.username,
            form.email,
            form.first_name,
            form.last_name,
            vec![String::from("That username is already in use.")],
            None,
        );
    }

    if state.lldap.email_exists(&form.email).await.unwrap_or(false) {
        return invite_form_view(
            cx,
            theme_class,
            code,
            form.username,
            form.email,
            form.first_name,
            form.last_name,
            vec![String::from("That email address is already in use.")],
            None,
        );
    }

    let token_hash = hash_token(&code);
    let invite_record = match state.invites.find_by_token_hash(&token_hash).await {
        Ok(Some(found_invite)) => found_invite,
        _ => {
            return invite_form_view(
                cx,
                theme_class,
                code,
                form.username,
                form.email,
                form.first_name,
                form.last_name,
                vec![String::from(
                    "This invitation link is invalid or has expired.",
                )],
                None,
            );
        }
    };

    if !invite_is_active(&invite_record, Utc::now()) {
        return invite_form_view(
            cx,
            theme_class,
            code,
            form.username,
            form.email,
            form.first_name,
            form.last_name,
            vec![String::from(
                "This invitation link is invalid or has expired.",
            )],
            None,
        );
    }

    let groups = match serde_json::from_str::<Vec<String>>(&invite_record.groups_json) {
        Ok(groups) => groups,
        Err(_) => {
            return invite_form_view(
                cx,
                theme_class,
                code,
                form.username,
                form.email,
                form.first_name,
                form.last_name,
                vec![String::from("This invitation could not be processed.")],
                None,
            );
        }
    };

    match state
        .lldap
        .provision_user(
            &form.username,
            &form.email,
            &form.first_name,
            &form.last_name,
            &form.password,
            &groups,
        )
        .await
    {
        Ok(()) => {}
        Err(error) => {
            let message = match error {
                LldapError::Http(_) | LldapError::Json(_) => {
                    String::from("The directory service could not be reached.")
                }
                LldapError::Graphql(_) => {
                    String::from("The account could not be created in the directory.")
                }
            };
            return invite_form_view(
                cx,
                theme_class,
                code,
                form.username,
                form.email,
                form.first_name,
                form.last_name,
                vec![message],
                None,
            );
        }
    }

    if !state
        .invites
        .consume_invite(&token_hash, &Utc::now().to_rfc3339(), &form.username)
        .await
        .unwrap_or(false)
    {
        let _ = state.lldap.delete_user_account(&form.username).await;
        return invite_form_view(
            cx,
            theme_class,
            code,
            form.username,
            form.email,
            form.first_name,
            form.last_name,
            vec![String::from(
                "The invitation could not be marked as used. Please try again.",
            )],
            None,
        );
    }

    invite_form_view(
        cx,
        theme_class,
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        Vec::new(),
        Some(String::from("Your account has been created.")),
    )
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::{InviteRecord, invite_is_active, invite_status};

    fn make_record(
        expires_at: &str,
        consumed_at: Option<&str>,
        revoked_at: Option<&str>,
    ) -> InviteRecord {
        InviteRecord {
            id: String::from("invite-1"),
            token_hash: String::from("hash-1"),
            groups_json: String::from("[\"engineering\"]"),
            created_at: String::from("2026-09-12T10:00:00Z"),
            expires_at: expires_at.to_owned(),
            consumed_at: consumed_at.map(str::to_owned),
            consumed_by: consumed_at.map(|_| String::from("new-user")),
            revoked_at: revoked_at.map(str::to_owned),
        }
    }

    #[test]
    fn invite_status_reports_all_states() {
        let now = Utc
            .with_ymd_and_hms(2026, 9, 12, 12, 0, 0)
            .single()
            .expect("valid timestamp");

        assert_eq!(
            invite_status(&make_record("2026-09-12T14:00:00Z", None, None), now,),
            "Active"
        );
        assert_eq!(
            invite_status(&make_record("2026-09-12T09:00:00Z", None, None), now,),
            "Expired"
        );
        assert_eq!(
            invite_status(
                &make_record("2026-09-12T14:00:00Z", Some("2026-09-12T11:00:00Z"), None),
                now,
            ),
            "Used"
        );
        assert_eq!(
            invite_status(
                &make_record("2026-09-12T14:00:00Z", None, Some("2026-09-12T11:00:00Z")),
                now,
            ),
            "Revoked"
        );
        assert_eq!(
            invite_status(&make_record("not-a-timestamp", None, None), now),
            "Invalid"
        );
    }

    #[test]
    fn invite_is_active_rejects_used_expired_revoked_and_invalid_records() {
        let now = Utc
            .with_ymd_and_hms(2026, 9, 12, 12, 0, 0)
            .single()
            .expect("valid timestamp");

        assert!(invite_is_active(
            &make_record("2026-09-12T14:00:00Z", None, None),
            now,
        ));
        assert!(!invite_is_active(
            &make_record("2026-09-12T09:00:00Z", None, None),
            now,
        ));
        assert!(!invite_is_active(
            &make_record("2026-09-12T14:00:00Z", Some("2026-09-12T11:00:00Z"), None),
            now,
        ));
        assert!(!invite_is_active(
            &make_record("2026-09-12T14:00:00Z", None, Some("2026-09-12T11:00:00Z")),
            now,
        ));
        assert!(!invite_is_active(
            &make_record("not-a-timestamp", None, None),
            now
        ));
    }
}
