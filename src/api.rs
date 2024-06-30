use actix_web::{get, post, web, HttpResponse, Responder, HttpRequest};
use serde::Deserialize;

pub mod exec;
pub mod model;
pub mod resolver;

#[derive(Deserialize)]
pub struct PathInfo {
    path: String
}

#[get("/api/kolibri/exec/{path:.*}")]
pub async fn exec_get(req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().body(format!("{}", req.path()))
}

#[post("/api/kolibri/exec/{path:.*}")]
pub async fn exec_post(path: web::Path<PathInfo>, body: String) -> impl Responder {
    HttpResponse::Ok().body(format!("{} {}", path.path, body))
}