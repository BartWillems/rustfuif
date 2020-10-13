use crate::auth;
use crate::ddg::Client;
use crate::server::Response;

use actix_identity::Identity;
use actix_web::{get, web};

#[derive(Deserialize)]
struct Query {
    query: String,
}

#[get("/images")]
async fn images(id: Identity, query: web::Query<Query>) -> Response {
    auth::get_user(&id)?;

    let res = Client::search_images(query.query.as_str()).await?;

    http_ok_json!(res)
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(images);
}
