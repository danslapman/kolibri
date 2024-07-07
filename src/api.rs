use crate::error::Error;
use crate::model::HttpMethod;
use crate::model::persistent::HttpStubResponse;
use actix_http::header::HeaderMap;
use actix_web::{get, head, post, put, delete, options, patch, HttpResponse, HttpRequest, Responder, ResponseError, Result};
use actix_web::web::{Bytes, Data, Query};
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
pub async fn exec_get(req: HttpRequest, body_bytes: Bytes, exec_handler: Data<ExecHandler>) -> Result<impl Responder> {
    let resp = exec_handler.get_ref().exec(
        HttpMethod::Get, 
        req.path().strip_prefix("/api/kolibri/exec").unwrap_or("").to_string(), 
        headermap_to_hashmap(req.headers()), 
        query_string_to_json_value(req.query_string())?, 
        bytes_to_request_body(body_bytes)?
    ).await?;

    if let Some(delay) = resp.get_delay() {
        sleep(delay.clone()).await;
    }

    Ok(response_to_responder(resp))
}

#[head("/api/kolibri/exec/{path:.*}")]
pub async fn exec_head(req: HttpRequest, body_bytes: Bytes, exec_handler: Data<ExecHandler>) -> Result<impl Responder> {
    let resp = exec_handler.get_ref().exec(
        HttpMethod::Head, 
        req.path().strip_prefix("/api/kolibri/exec").unwrap_or("").to_string(), 
        headermap_to_hashmap(req.headers()), 
        query_string_to_json_value(req.query_string())?, 
        bytes_to_request_body(body_bytes)?
    ).await?;

    if let Some(delay) = resp.get_delay() {
        sleep(delay.clone()).await;
    }

    Ok(response_to_responder(resp))
}

#[post("/api/kolibri/exec/{path:.*}")]
pub async fn exec_post(req: HttpRequest, body_bytes: Bytes, exec_handler: Data<ExecHandler>) -> Result<impl Responder> {
    let resp = exec_handler.get_ref().exec(
        HttpMethod::Get, 
        req.path().strip_prefix("/api/kolibri/exec").unwrap_or("").to_string(), 
        headermap_to_hashmap(req.headers()), 
        query_string_to_json_value(req.query_string())?,
        bytes_to_request_body(body_bytes)?
    ).await?;

    if let Some(delay) = resp.get_delay() {
        sleep(delay.clone()).await;
    }

    Ok(response_to_responder(resp))
}

#[put("/api/kolibri/exec/{path:.*}")]
pub async fn exec_put(req: HttpRequest, body_bytes: Bytes, exec_handler: Data<ExecHandler>) -> Result<impl Responder> {
    let resp = exec_handler.get_ref().exec(
        HttpMethod::Put, 
        req.path().strip_prefix("/api/kolibri/exec").unwrap_or("").to_string(), 
        headermap_to_hashmap(req.headers()), 
        query_string_to_json_value(req.query_string())?,
        bytes_to_request_body(body_bytes)?
    ).await?;

    if let Some(delay) = resp.get_delay() {
        sleep(delay.clone()).await;
    }

    Ok(response_to_responder(resp))
}

#[delete("/api/kolibri/exec/{path:.*}")]
pub async fn exec_delete(req: HttpRequest, body_bytes: Bytes, exec_handler: Data<ExecHandler>) -> Result<impl Responder> {
    let resp = exec_handler.get_ref().exec(
        HttpMethod::Delete, 
        req.path().strip_prefix("/api/kolibri/exec").unwrap_or("").to_string(), 
        headermap_to_hashmap(req.headers()), 
        query_string_to_json_value(req.query_string())?, 
        bytes_to_request_body(body_bytes)?
    ).await?;

    if let Some(delay) = resp.get_delay() {
        sleep(delay.clone()).await;
    }

    Ok(response_to_responder(resp))
}

#[options("/api/kolibri/exec/{path:.*}")]
pub async fn exec_options(req: HttpRequest, body_bytes: Bytes, exec_handler: Data<ExecHandler>) -> Result<impl Responder> {
    let resp = exec_handler.get_ref().exec(
        HttpMethod::Options, 
        req.path().strip_prefix("/api/kolibri/exec").unwrap_or("").to_string(), 
        headermap_to_hashmap(req.headers()), 
        query_string_to_json_value(req.query_string())?, 
        bytes_to_request_body(body_bytes)?
    ).await?;

    if let Some(delay) = resp.get_delay() {
        sleep(delay.clone()).await;
    }

    Ok(response_to_responder(resp))
}

#[patch("/api/kolibri/exec/{path:.*}")]
pub async fn exec_patch(req: HttpRequest, body_bytes: Bytes, exec_handler: Data<ExecHandler>) -> Result<impl Responder> {
    let resp = exec_handler.get_ref().exec(
        HttpMethod::Patch, 
        req.path().strip_prefix("/api/kolibri/exec").unwrap_or("").to_string(), 
        headermap_to_hashmap(req.headers()), 
        query_string_to_json_value(req.query_string())?,
        bytes_to_request_body(body_bytes)?
    ).await?;

    if let Some(delay) = resp.get_delay() {
        sleep(delay.clone()).await;
    }

    Ok(response_to_responder(resp))
}

// ---- private stuff ----

impl ResponseError for Error { }

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

fn query_string_to_json_value(query_string: &str) -> Result<Value, Error> {
    let params = Query::<Vec<(String, String)>>::from_query(query_string)
        .map_err(|e| Error::from(e))?.0;

    Ok(Value::from_iter(params.into_iter().map(|(key, value)| (key, serde_json::from_str(value.as_str()).unwrap_or(Value::String(value)) ))))
}

fn bytes_to_request_body(body_bytes: Bytes) -> Result<RequestBody, Error> {
    if body_bytes.is_empty() {
        Ok(RequestBody::AbsentRequestBody)
    } else {
        let bytes_vec = body_bytes.to_vec();
        String::from_utf8(bytes_vec.clone())
            .map_err(|e| Error::from(e))
            .map(|body_str| RequestBody::SimpleRequestBody { raw_value: bytes_vec, value: body_str })
    }
}