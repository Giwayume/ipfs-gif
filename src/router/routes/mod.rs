use std::path::PathBuf;

use axum::{
    routing::{ get, get_service },
    Router,
};
use tower_http::{
    services::{ ServeDir },
};

pub mod api;
pub mod dcma;
pub mod gif;
pub mod home;
pub mod page_not_found;
pub mod privacy_policy;
pub mod terms_of_service;
pub mod upload;

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

        .route("/api", get(api::get_api))
        .route("/api/", get(api::get_api))

        .route("/dcma", get(dcma::get_dcma))
        .route("/dcma/", get(dcma::get_dcma))

        .route("/gif/{cid}", get(gif::get_gif))
        .route("/gif/{cid}/", get(gif::get_gif))

        .route("/privacy-policy", get(privacy_policy::get_privacy_policy))
        .route("/privacy-policy/", get(privacy_policy::get_privacy_policy))

        .route("/terms-of-service", get(terms_of_service::get_terms_of_service))
        .route("/terms-of-service/", get(terms_of_service::get_terms_of_service))

        .route("/upload", get(upload::get_upload))
        .route("/upload/", get(upload::get_upload))

        .fallback_service(get_service(ServeDir::new(uploaded_files_path)));

    app
}