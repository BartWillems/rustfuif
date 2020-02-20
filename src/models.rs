// use std::time::{Duration, SystemTime};

// create game with teams

// #[derive(Queryable)]
// pub struct Game {
//     pub id: i64,
//     pub name: String,
//     pub start_time: SystemTime,
//     pub duration: Duration,
//     pub teams: Vec<Team>,
//     pub beverage_slots: Vec<Slot>,
// }

#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct Team {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct Slot {
    pub id: i64,
    pub game_id: i64,
    // pub team_id: i64,
    // pub slot: i8, // --> composite primary key? // AS ROW_NUMBER() % (teams.length)
}

#[derive(Queryable)]
pub struct Beverage {
    pub id: i64,
    pub team_id: i64,
    pub slot_id: i64,
    pub name: String,
}

#[derive(Queryable)]
pub struct Price {
    pub id: i64,
    pub beverage_id: i64,
    pub default_price: u32,
    pub minimum_price: u32,
    pub maximum_price: u32,
}

// type beverage struct {
// 	id     int64
// 	teamID int64
// 	slot   int8
// 	name   string

// 	defaultPrice int // in cents, 100 = €1
// 	priceMin     int // in cents, 100 = €1
// 	priceMax     int // in cents, 100 = €1
// }

// type sale struct {
// 	id         int64
// 	teamID     int64
// 	beverageID int8
// }
