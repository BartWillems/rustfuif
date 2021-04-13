use actix_identity::Identity;
use actix_web::http::StatusCode;
use actix_web::web;
use actix_web::web::{Data, HttpResponse, Json, Path, Query};
use actix_web::{delete, get, post, put};

use crate::auth;
use crate::games::models::{Beverage, CreateGame, Game, GameFilter};
use crate::prices::PriceHistory;
use crate::server::{self, State};
use crate::validator::Validator;

#[get("/games")]
async fn find_all(query: Query<GameFilter>, state: Data<State>, id: Identity) -> server::Response {
    let user = auth::get_user(&id)?;

    let games;

    if user.is_admin {
        debug!("user is admin, showing all games");
        games = Game::find_all(query.into_inner(), &state.db).await?;
    } else {
        games = Game::find_by_user(user.id, query.into_inner(), &state.db).await?;
    }

    http_ok_json!(games);
}

#[get("/games/{id}")]
async fn find(game_id: Path<i64>, state: Data<State>, id: Identity) -> server::Response {
    let user = auth::get_user(&id)?;

    if !user.is_admin && !Game::verify_user_participation(*game_id, user.id, &state.db).await? {
        forbidden!("user is not in game");
    }
    let game = Game::find_by_id(*game_id, &state.db).await?;

    http_ok_json!(game);
}

#[post("/games")]
async fn create(
    game: Json<Validator<CreateGame>>,
    state: Data<State>,
    id: Identity,
) -> server::Response {
    let mut game = game.into_inner().validate()?;

    game.owner_id = auth::get_user(&id)?.id;

    let game = Game::create(game, &state.db).await?;

    http_created_json!(game);
}

#[put("/games")]
async fn update(game: Json<Game>, state: Data<State>, id: Identity) -> server::Response {
    let user = auth::get_user(&id)?;

    let old_game = Game::find_by_id(game.id, &state.db).await?;
    if old_game.owner_id != user.id && !user.is_admin {
        forbidden!("Only game owners can delete games");
    }

    let game = game.update(&state.db).await?;

    http_ok_json!(game);
}

#[delete("/games/{id}")]
async fn delete(game_id: Path<i64>, state: Data<State>, id: Identity) -> server::Response {
    let user = auth::get_user(&id)?;

    let game = Game::find_by_id(*game_id, &state.db).await?;
    if game.owner_id != user.id && !user.is_admin {
        forbidden!("Only game owners can delete games");
    }

    game.delete(&state.db).await?;

    Ok(HttpResponse::new(StatusCode::OK))
}

#[get("/games/{id}/beverages")]
async fn get_beverages(game_id: Path<i64>, state: Data<State>, id: Identity) -> server::Response {
    let user = auth::get_user(&id)?;

    let beverages = Beverage::find(*game_id, user.id, &state.db).await?;

    http_ok_json!(beverages);
}

#[post("/games/{id}/beverages")]
async fn create_beverage(
    game_id: Path<i64>,
    beverage: Json<Validator<Beverage>>,
    state: Data<State>,
    id: Identity,
) -> server::Response {
    let user = auth::get_user(&id)?;

    let game_id = *game_id;
    let mut beverage = beverage.into_inner().validate()?;

    beverage.user_id = user.id;
    beverage.game_id = game_id;
    beverage.set_price(beverage.starting_price);

    if !Game::verify_user_participation(game_id, user.id, &state.db).await? {
        forbidden!("you are not in this game");
    }

    let beverage = beverage.save(&state.db).await?;

    http_created_json!(beverage);
}

#[put("/games/{id}/beverages")]
async fn update_beverage_config(
    game_id: Path<i64>,
    config: Json<Validator<Beverage>>,
    state: Data<State>,
    id: Identity,
) -> server::Response {
    let user = auth::get_user(&id)?;

    let game_id = *game_id;
    let mut config = config.into_inner().validate()?;

    config.user_id = user.id;
    config.game_id = game_id;

    if !Game::verify_user_participation(game_id, user.id, &state.db).await? {
        forbidden!("you are not in this game");
    }

    let config = config.update(&state.db).await?;

    http_created_json!(config);
}

#[get("/games/{id}/stats/price-history")]
async fn price_history(game_id: Path<i64>, state: Data<State>, id: Identity) -> server::Response {
    let user = auth::get_user(&id)?;

    let prices = PriceHistory::load(user.id, *game_id, &state.db).await?;

    http_ok_json!(prices);
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(find_all);
    cfg.service(find);
    cfg.service(create);
    cfg.service(update);
    cfg.service(delete);

    cfg.service(create_beverage);
    cfg.service(get_beverages);
    cfg.service(update_beverage_config);

    cfg.service(price_history);
}
