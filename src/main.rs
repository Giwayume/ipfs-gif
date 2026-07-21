use std::env;
use std::net::SocketAddr;

mod database;
mod router;
mod ui_pages;
mod ui_primitives;
mod util;

#[tokio::main]
async fn main() {
    unsafe {
        env::set_var("MEMORY_SERVE_QUIET", "1");
    }

    let secrets_config = util::secrets::secrets_config();

    let _ = util::tracing::init_tracing();
    let _ = database::init_pool().await;
    let _ = database::initialize::create_all_tables().await;
    let _ = router::authn::init_moderator_sessions().await;
    let _ = util::smtp::init_mailer();
    util::image_scan::start_scanning_quarantine();
    tokio::spawn(util::image_upload::init_temporary_image_upload_cleanup());

    let app = router::initialize();

    let listener = tokio::net::TcpListener::bind(
        format!("0.0.0.0:{}", secrets_config.website.port)
    )
        .await
        .unwrap();

    tracing::info!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    ).await.unwrap();
}
