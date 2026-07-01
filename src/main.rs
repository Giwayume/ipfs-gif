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

    let _ = util::tracing::init_tracing();
    let _ = database::init_pool().await;
    let _ = database::initialize::create_all_tables().await;
    tokio::spawn(util::image_upload::init_temporary_image_upload_cleanup());

    let app = router::initialize();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3040")
        .await
        .unwrap();

    tracing::info!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    ).await.unwrap();
}
