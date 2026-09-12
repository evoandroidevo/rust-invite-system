use topcoat::{
    Result,
    router::{Router, RouterBuilderDiscoverExt, page},
    view::{View, view},
};

#[tokio::main]
async fn main() {
    topcoat::start(Router::builder().discover().build())
        .await
        .unwrap();
}

#[page("/")]
async fn home() -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <title>"Rust Invite System"</title>
            </head>
            <body>
                <h1>"Rust Invite System"</h1>
            </body>
        </html>
    })
}
