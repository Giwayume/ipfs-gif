use std::path::PathBuf;

use axum::{
    routing::{ get, get_service },
    Router,
};
use tower_http::{
    services::{ ServeDir },
};

pub mod gif;
pub mod home;
pub mod page_not_found;

pub fn initialize() -> Router {
    // Static assets
    let memory_router = memory_serve::load!()
        .into_router();

    // Uploaded assets
    let uploaded_files_path = PathBuf::from("uploads");

    let app = Router::new()
        .merge(memory_router)
        .route("/", get(home::get_home))

        .route("/404", get(page_not_found::get_page_not_found))
        .route("/404/", get(page_not_found::get_page_not_found))

        .route("/gif/{cid}", get(gif::get_gif))
        .route("/gif/{cid}/", get(gif::get_gif))

        .fallback_service(get_service(ServeDir::new(uploaded_files_path)));

    app
}