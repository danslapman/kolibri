use crate::error::Error;
use crate::model::HttpMethod;
use crate::model::persistent::HttpStubResponse;
use actix_http::header::HeaderMap;
use actix_web::{get, post, HttpResponse, HttpRequest, Responder, ResponseError, Result};
use actix_web::web::Data;
use exec::ExecHandler;
use http::StatusCode;
use model::RequestBody;
use serde_json::Value;
use std::collections::HashMap;
use tokio::time::sleep;

pub mod exec;
pub mod model;
pub mod resolver;

#[get("/api/kolibri/exec/{path:.*}")]
pub async fn exec_get(req: HttpRequest, exec_handler: Data<ExecHandler>) -> Result<impl Responder> {
    let resp = exec_handler.get_ref().exec(
        HttpMethod::Get, 
        req.path().strip_prefix("/api/kolibri/exec").unwrap_or("").to_string(), 
        headermap_to_hashmap(req.headers()), 
        Value::Null, 
        RequestBody::AbsentRequestBody
    )?;

    if let Some(delay) = resp.get_delay() {
        sleep(delay.clone()).await;
    }

    Ok(response_to_responder(resp))
}

#[post("/api/kolibri/exec/{path:.*}")]
pub async fn exec_post(req: HttpRequest, exec_handler: Data<ExecHandler>) -> Result<impl Responder> {
    let resp = exec_handler.get_ref().exec(
        HttpMethod::Get, 
        req.path().strip_prefix("/api/kolibri/exec").unwrap_or("").to_string(), 
        headermap_to_hashmap(req.headers()), 
        Value::Null, 
        RequestBody::AbsentRequestBody
    )?;

    if let Some(delay) = resp.get_delay() {
        sleep(delay.clone()).await;
    }

    Ok(response_to_responder(resp))
}

fn response_to_responder(stub_response: HttpStubResponse) -> impl Responder {
    match stub_response {
        HttpStubResponse::RawResponse { code, headers, body, .. } => {
            let mut builder = HttpResponse::build(StatusCode::from_u16(code).unwrap());

            for (key, value) in headers.into_iter() {
                builder.append_header((key, value));
            }

            builder.body(body)
        },
        HttpStubResponse::JsonResponse { code, headers, body, .. } => {
            let mut builder = HttpResponse::build(StatusCode::from_u16(code).unwrap());

            for (key, value) in headers.into_iter() {
                builder.append_header((key, value));
            }

            builder.body(body.to_string())
        }
    }
}

fn headermap_to_hashmap(headermap: &HeaderMap) -> HashMap<String, String> {
    headermap
        .into_iter()
        .map(|(name, value)| (name.as_str().to_string(), format!("{:?}", value)))
        .collect()
}

impl ResponseError for Error { }