use topcoat::{
    Result,
    asset::{Asset, asset},
    context::{Cx, app_context},
    router::{page, query_params},
    view::{View, view},
};

use crate::app_state::AppState;
use crate::views;

#[query_params(error = bad_request)]
struct InviteQuery {
    code: Option<String>,
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

const THEME_LIGHT_ICON: Asset = asset!("./assets/icons/theme-light.svg");
const THEME_DARK_ICON: Asset = asset!("./assets/icons/theme-dark.svg");

const THEME_SCRIPT: &str = r#"
    const body = document.body;
    const toggle = document.getElementById('theme-toggle');
    const savedTheme = localStorage.getItem('invite-theme');
    if (savedTheme === 'light' || savedTheme === 'dark') {
        body.className = `signup-shell theme-${savedTheme}`;
    }
    function updateThemeLabel() {
        const isDark = body.classList.contains('theme-dark');
        toggle.querySelector('.theme-label').textContent = isDark ? 'Use light mode' : 'Use dark mode';
        toggle.setAttribute('aria-pressed', String(isDark));
    }
    updateThemeLabel();
    toggle.addEventListener('click', function () {
        const nextTheme = body.classList.contains('theme-dark') ? 'light' : 'dark';
        body.className = `signup-shell theme-${nextTheme}`;
        localStorage.setItem('invite-theme', nextTheme);
        updateThemeLabel();
    });
"#;

#[page("/")]
async fn home() -> Result<impl View> {
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
            </body>
        </html>
    })
}

#[page("/health")]
async fn health(cx: &Cx) -> Result<impl View> {
    let state: &AppState = app_context(cx);

    Ok(view! {
        <p>
            "ok (invite expiry: " (state.config.invites.expiration_hours) " hours)"
        </p>
    })
}

#[page("/invite")]
async fn invite(cx: &Cx) -> Result<impl View> {
    let query = query_params::<InviteQuery>(cx)?;
    let code = query.code.clone().unwrap_or_default();
    let state: &AppState = app_context(cx);
    let theme_class = state.config.appearance.default_theme.css_class();

    Ok(view! {
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
                        <form action="/invite/submit" method="post">
                            <input type="hidden" name="code" value=(code) />
                            <label>
                                "Username"
                                <input type="text" name="username" autocomplete="username" required=(true) />
                            </label>
                            <label>
                                "Email"
                                <input type="email" name="email" autocomplete="email" required=(true) />
                            </label>
                            <div class="field-row">
                                <label>
                                    "First name"
                                    <input type="text" name="first_name" autocomplete="given-name" required=(true) />
                                </label>
                                <label>
                                    "Last name"
                                    <input type="text" name="last_name" autocomplete="family-name" required=(true) />
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
