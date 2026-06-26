/**
 * The purpose of this file is to generate a big "context" struct
 * that contains all the necessary information about a HTTP request
 * in order for the askama templates to be able to render the page.
 *
 * Since some templates (like pagination) are buried deep in the call
 * stack and need to know things like query parameters, instead of 
 * changing the function arguments every time I realize new data is needed,
 * everything about the request is added to this context struct.
 */

use std::{
    collections::HashMap,
    net::SocketAddr,
};
use axum::{
    body::{ Bytes, Body },
    extract::{ ConnectInfo, FromRequest, FromRequestParts, Multipart, OriginalUri, Path, Query },
    http::{
        HeaderMap, Request, Uri,
    },
};
use garde::Report;
use regex::Regex;
use serde_urlencoded;
use urlencoding::encode;

use crate::util::image_upload;

const BODY_SIZE_LIMIT: usize = 1024 * 10; // 10 KB

 /**
 * Utilities for accessing a hash map as query parameters
 */

pub trait QueryHashMap {
    // fn get_as<T: FromStr>(&self, key: &str) -> Option<T>;
    fn to_query_string(&self) -> String;
}
impl QueryHashMap for HashMap<String, String> {
    // fn get_as<T: FromStr>(&self, key: &str) -> Option<T> {
    //     self.get(key)
    //         .and_then(|value| value.parse::<T>().ok())
    // }
    
    fn to_query_string(&self) -> String {
        self.into_iter()
            .map(|(key, value)| format!("{}={}", encode(&key), encode(&value)))
            .collect::<Vec<String>>()
            .join("&")
    }
}


/**
 * Traits to extract information about the route (url, query) from the context object
 */

#[allow(unused)]
pub trait RouteContext {
    fn route_headers(&self) -> &HeaderMap;
    fn route_original_uri(&self) -> &Uri;
    fn route_query(&self) -> &HashMap<String, String>;
}

pub trait RouteParamContextGenerator {
    type Type;
    fn populate_from_context_extractor(
        path: &HashMap<String, String>,
        query: &HashMap<String, String>,
        form: &HashMap<String, String>,
    ) -> Self::Type;
}

/**
 * Traits to extract global state; used by components that appear on multiple pages
 */

#[allow(unused)]
pub trait GlobalStateContext {
    fn validation_reports(&self) -> &HashMap<String, Report>;
}

/**
 * Base context object that is passed to askama templates
 */

#[allow(unused)]
#[derive(Default)]
pub struct BaseContext<P> {
    pub params: P,
    pub ip_address: String,
    pub route_headers: HeaderMap,
    pub route_original_uri: Uri,
    pub route_path_params: HashMap<String, String>,
    pub route_query: HashMap<String, String>,
    pub validation_reports: HashMap<String, Report>,
}
impl<P> BaseContext<P> {
    pub fn clone_with_params<C>(&self, params: C) -> BaseContext<C> {
        BaseContext::<C> {
            params,
            ip_address: self.ip_address.clone(),
            route_headers: self.route_headers.clone(),
            route_original_uri: self.route_original_uri.clone(),
            route_path_params: self.route_path_params.clone(),
            route_query: self.route_query.clone(),
            validation_reports: self.validation_reports.clone(),
        }
    }
}
#[allow(unused)]
impl<P> RouteContext for &BaseContext<P> {
    fn route_headers(&self) -> &HeaderMap { &self.route_headers }
    fn route_original_uri(&self) -> &Uri { &self.route_original_uri }
    fn route_query(&self) -> &HashMap<String, String> { &self.route_query }
}
#[allow(unused)]
impl<P> GlobalStateContext for &BaseContext<P> {
    fn validation_reports(&self) -> &HashMap<String, Report> { &self.validation_reports }
}


/**
 * Axum extractor to generate the context struct for a route
 */

pub struct Context<P> {
    pub context: BaseContext<P>,
}
impl<S, P> FromRequest<S> for Context<P>
where
    Bytes: FromRequest<S>,
    P: RouteParamContextGenerator<Type = P>,
    S: Send + Sync,
{
    type Rejection = axum::response::Response;

    async fn from_request(request: Request<Body>, state: &S) -> Result<Self, Self::Rejection> {
        let (mut parts, body) = request.into_parts();

        let route_path_params: HashMap<String, String> = Path::from_request_parts(& mut parts, state).await
            .map_err(|_| axum::response::Response::builder().status(400).body("Invalid path".into()).unwrap())?
            .0;

        let route_query: HashMap<String, String> = Query::from_request_parts(& mut parts, state).await
            .map_err(|_| axum::response::Response::builder().status(400).body("Invalid query".into()).unwrap())?
            .0;

        let route_original_uri = OriginalUri::from_request_parts(& mut parts, state).await
            .map_err(|_| axum::response::Response::builder().status(400).body("Invalid original uri".into()).unwrap())?
            .0;

        let ip_address: String = match parts.headers.get("X-Forwarded-For") {
            Some(x_forwarded_for) => {
                match x_forwarded_for.to_str() {
                    Ok(x_forwarded_for) => x_forwarded_for.to_string(),
                    _ => String::from(""),
                }
            },
            _ => {
                format!("{}", parts
                    .extensions
                    .get::<ConnectInfo<SocketAddr>>()
                    .map(|c| c.0)
                    .unwrap_or_else(|| {
                        "0.0.0.0:0".parse().unwrap()
                    })
                    .ip()
                )
            },
        };

        let mut route_body: HashMap<String, String> = HashMap::new();
        let mut route_body_multi_fields: HashMap<String, Vec<String>> = HashMap::new();
        if let Some(content_type_header) = parts.headers.get("Content-Type") {
            match content_type_header.to_str() {
                Ok(content_type) => {
                    let mime_type = content_type.split(';').collect::<Vec<&str>>().first().unwrap().trim();
                    let multi_field_regex = Regex::new(r"\[([0-9]*?)\]$").unwrap();
                    match mime_type {
                        "application/x-www-form-urlencoded" => {
                            let body_bytes = axum::body::to_bytes(body, BODY_SIZE_LIMIT).await.unwrap();
                            route_body = serde_urlencoded::from_bytes(&body_bytes).unwrap();
                        },
                        "multipart/form-data" => {
                            let new_request = Request::from_parts(parts.clone(), body);
                            let mut multipart = Multipart::from_request(new_request, &None::<Option<i32>>).await.unwrap();
                            loop {
                                match multipart.next_field().await {
                                    Ok(field_option) => {
                                        match field_option {
                                            Some(field) => {
                                                let name = field.name().unwrap().to_string();
                                                match field.content_type() {
                                                    Some(content_type) => {
                                                        let content_type_copy = content_type.to_string();
                                                        if let Ok(bytes) = field.bytes().await {
                                                            if let Ok(file_name) = image_upload::store_temporary_image(content_type_copy, bytes).await {
                                                                route_body.insert(name, file_name);
                                                            }
                                                        }
                                                    },
                                                    None => {
                                                        if let Ok(text) = field.text().await {
                                                            if multi_field_regex.is_match(&name) {
                                                                let field_name = multi_field_regex.replace_all(&name, "").into_owned();
                                                                if !route_body_multi_fields.contains_key(field_name.as_str()) {
                                                                    route_body_multi_fields.insert(field_name.clone(), Vec::new());
                                                                }
                                                                let field_value = route_body_multi_fields
                                                                    .entry(field_name.clone())
                                                                    .or_insert_with(Vec::new);
                                                                if let Some(captures) = multi_field_regex.captures(&name) {
                                                                    if let Some(number_match) = captures.get(1) {
                                                                        let number_str = number_match.as_str();
                                                                        if let Ok(index) = number_str.parse::<usize>() {
                                                                            let expected_length = index + 1;
                                                                            if field_value.len() < expected_length {
                                                                                field_value.resize(expected_length, String::from(""));
                                                                            }
                                                                            field_value.insert(index, text.replace(",", "%2C"));
                                                                        }
                                                                    }
                                                                }
                                                            } else {
                                                                route_body.insert(name, text);
                                                            }
                                                        }
                                                    },
                                                }
                                            },
                                            None => {
                                                break;
                                            },
                                        }
                                    },
                                    Err(error) => {
                                        tracing::warn!("Failed to parse multipart/form-data request body. {:?}", error);
                                        break;
                                    }
                                }
                            }
                        },
                        _ => (),
                    }
                }
                Err(_) => (),
            }
        }
        for (key, value) in &route_body_multi_fields {
            route_body.insert(
                key.clone(),
                format!("{}", value.join(",")),
            );
        }

        let params: P = P::populate_from_context_extractor(&route_path_params, &route_query, &route_body);

        let route_headers: HeaderMap = parts.headers;

        Ok(Context {
            context: BaseContext {
                params,
                ip_address,
                route_headers,
                route_original_uri,
                route_path_params,
                route_query,
                validation_reports: HashMap::new(),
            },
        })
    }
}