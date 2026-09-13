mod app_state;
mod configuration;
mod invite_storage;
mod lldap;
mod routes;
mod validation;
mod views;

use std::path::PathBuf;
use std::time::Duration as StdDuration;

use chrono::{Duration, Utc};
use topcoat::{
    asset::{AssetBundle, RouterBuilderAssetExt},
    router::{Router, RouterBuilderDiscoverExt},
};

use crate::invite_storage::STALE_INVITE_RETENTION_DAYS;
use crate::{app_state::AppState, configuration::AppConfig};

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
fn spawn_invite_cleanup_task(invites: crate::invite_storage::InviteRepository) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(StdDuration::from_secs(24 * 60 * 60));
        loop {
            interval.tick().await;
            let cutoff = Utc::now() - Duration::days(STALE_INVITE_RETENTION_DAYS);
            if let Err(error) = invites.cleanup_stale(&cutoff.to_rfc3339()).await {
                eprintln!("invite cleanup failed: {error}");
            }
        }
    });
}

#[tokio::main]
async fn main() {
    sync_assets().expect("assets must be bundled");
    let config = AppConfig::load("config.toml").expect("configuration must be valid");
    std::fs::create_dir_all("data").expect("database directory must be available");
    let state = AppState::initialize(config)
        .await
        .expect("application state must initialize");
    spawn_invite_cleanup_task(state.invites.clone());
    let router = Router::builder()
        .discover()
        .assets(AssetBundle::load().expect("asset bundle must be available"))
        .app_context(state)
        .build();

    topcoat::start(router).await.unwrap();
}
