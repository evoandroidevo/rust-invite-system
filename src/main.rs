mod app_state;
mod configuration;
mod invite_storage;
mod lldap;
mod routes;
mod validation;
mod views;

use topcoat::{
    asset::{AssetBundle, RouterBuilderAssetExt},
    router::{Router, RouterBuilderDiscoverExt},
};

use crate::{app_state::AppState, configuration::AppConfig};

#[tokio::main]
async fn main() {
    let config = AppConfig::load("config.toml").expect("configuration must be valid");
    std::fs::create_dir_all("data").expect("database directory must be available");
    let state = AppState::initialize(config)
        .await
        .expect("application state must initialize");
    let router = Router::builder()
        .discover()
        .assets(AssetBundle::load().expect("asset bundle must be available"))
        .app_context(state)
        .build();

    topcoat::start(router).await.unwrap();
}
