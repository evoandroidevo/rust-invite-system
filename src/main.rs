use std::path::PathBuf;
use std::time::Duration as StdDuration;

use chrono::{Duration, Utc};
use topcoat::{
    asset::{AssetBundle, RouterBuilderAssetExt},
    cookie::RouterBuilderCookieExt,
    router::{Router, RouterBuilderDiscoverExt},
};

use rust_invite_system::invite_storage::STALE_INVITE_RETENTION_DAYS;
use rust_invite_system::{app_state::AppState, configuration::AppConfig};

fn sync_assets() -> Result<(), Box<dyn std::error::Error>> {
    let executable = std::env::current_exe()?;
    let bundle_dir = executable
        .parent()
        .ok_or("executable directory is unavailable")?
        .join("assets");
    let binary = std::fs::read(executable)?;
    topcoat_asset::Bundler::new(
        &topcoat_asset::BundlerConfig::new().cache_dir(PathBuf::from("target/topcoat-cache")),
    )
    .bundle(&binary, bundle_dir)?;
    Ok(())
}

/// Periodically deletes expired/revoked invites older than the retention window.
fn spawn_invite_cleanup_task(invites: rust_invite_system::invite_storage::InviteRepository) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(StdDuration::from_secs(24 * 60 * 60));
        loop {
            interval.tick().await;
            let cutoff = Utc::now() - Duration::days(STALE_INVITE_RETENTION_DAYS);
            if let Err(error) = invites.cleanup_stale(&cutoff.to_rfc3339()).await {
                let _ = error;
                eprintln!("invite cleanup failed");
            }
        }
    });
}

#[tokio::main]
async fn main() {
    if std::env::args().nth(1).as_deref() == Some("--hash-admin-password") {
        if let Err(error) = provision_admin_password() {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    sync_assets().expect("assets must be bundled");
    let config = AppConfig::load("config.toml").expect("configuration must be valid");
    std::fs::create_dir_all("data").expect("database directory must be available");
    let state = AppState::initialize(config)
        .await
        .expect("application state must initialize");
    spawn_invite_cleanup_task(state.invites.clone());
    let router = Router::builder()
        .discover()
        .cookies()
        .assets(AssetBundle::load().expect("asset bundle must be available"))
        .app_context(state)
        .build();

    topcoat::start(router).await.unwrap();
}

fn provision_admin_password() -> Result<(), Box<dyn std::error::Error>> {
    let password = rpassword::prompt_password("Admin password (16+ bytes): ")?;
    let confirmation = rpassword::prompt_password("Confirm admin password: ")?;
    if password != confirmation {
        return Err("Passwords do not match".into());
    }
    println!(
        "{}",
        rust_invite_system::admin_auth::hash_password(&password)?
    );
    Ok(())
}
