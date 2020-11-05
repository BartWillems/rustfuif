use std::collections::HashMap;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::result::Error as DBError;

use crate::db;
use crate::errors::ServiceError;
use crate::games::{Beverage, Game};
use crate::schema::{beverages, sales_counts, transactions, users};

#[derive(Debug, Serialize, Queryable, Identifiable, AsChangeset, Clone)]
pub struct Transaction {
    pub id: i64,
    pub user_id: i64,
    pub game_id: i64,
    pub slot_no: i16,
    pub created_at: Option<DateTime<Utc>>,
    pub amount: i32,
    pub price: i64,
}

#[derive(Debug, Deserialize)]
pub struct TransactionFilter {
    pub user_id: Option<i64>,
    pub game_id: Option<i64>,
}

#[derive(Debug, Deserialize, Insertable, Clone, Copy)]
#[table_name = "transactions"]
pub struct Sale {
    pub user_id: i64,
    pub game_id: i64,
    pub slot_no: i16,
    pub amount: i32,
    pub price: i64,
}

#[derive(Debug, Deserialize)]
pub struct NewSale {
    pub user_id: i64,
    pub game_id: i64,
    pub slots: HashMap<i16, i32>,
}

/// contains how many sales have been made for a given slot
#[derive(Serialize, Queryable, Clone, Copy, Debug, Default)]
pub struct SlotSale {
    pub slot_no: i16,
    pub sales: i64,
}

#[derive(Serialize, Queryable)]
pub struct UserSales {
    pub username: String,
    pub sales: i64,
}

#[derive(Debug, Serialize, Deserialize, Insertable, Queryable)]
pub struct SalesCount {
    pub game_id: i64,
    pub slot_no: i16,
    pub sales: i64,
}

impl NewSale {
    pub fn save(&self, conn: &db::Conn) -> Result<Vec<Transaction>, ServiceError> {
        let transactions = conn.transaction::<Vec<Transaction>, ServiceError, _>(|| {
            // NEW SALES ORDER
            // 1. Fetch beverage configs FOR UPDATE
            // 2. Fetch current sales_counts FOR UPDATE
            // 3. Calculate the prices for each beverage in the new sale
            //    - loop over sales
            //    - fetch beverage config based on slot_no by looping
            //    - get price
            //    - create new transaction struct dink
            // 4. update sales_counts
            // 5. insert in transactions with the current count

            let game = Game::find_by_id(self.game_id, conn)?;

            for slot_no in self.slots.keys() {
                if slot_no > &game.beverage_count || slot_no < &0 {
                    bad_request!("a beverage slot exceeds the maximum configured beverage slots");
                }
            }

            let mut sales: HashMap<i16, Sale> = self.unroll()?;
            use diesel::dsl::any;

            let keys: Vec<&i16> = sales.keys().collect();
            // 1
            let beverage_configs = beverages::table
                .filter(beverages::user_id.eq(self.user_id))
                .filter(beverages::game_id.eq(self.game_id))
                .filter(beverages::slot_no.eq(any(keys)))
                .for_update()
                .load::<Beverage>(conn)?;

            // 2
            let mut sales_counts = SalesCount::find_by_game_for_update(self.game_id, conn)?;

            // 3
            for (_, sale) in sales.iter_mut() {
                let mut beverage_config: Option<&Beverage> = None;
                for cfg in &beverage_configs {
                    if cfg.slot_no == sale.slot_no {
                        beverage_config = Some(cfg);
                        break;
                    }
                }
                match beverage_config {
                    None => {
                        error!("a sale was attempted without a pre-existing beverage config");
                        return Err(ServiceError::BadRequest(String::from(
                            "unable to create purchase for beverage without a config",
                        )));
                    }
                    Some(beverage) => {
                        sale.set_price(beverage);
                    }
                }
            }

            // 4
            for sale_count in sales_counts.iter_mut() {
                if let Some(sale) = sales.get(&sale_count.slot_no) {
                    sale_count.sales += sale.amount as i64;
                    sale_count.update(conn)?;
                }
            }

            // 5
            let sales: Vec<&Sale> = sales.values().collect();

            let transactions = diesel::insert_into(transactions::table)
                .values(sales)
                .get_results::<Transaction>(conn)?;

            Ok(transactions)
        })?;

        Ok(transactions)
    }

    /// turn the map of slots to a map of sales with their slot no as key
    fn unroll(&self) -> Result<HashMap<i16, Sale>, ServiceError> {
        let mut sales: HashMap<i16, Sale> = HashMap::new();

        for (slot_no, amount) in &self.slots {
            let sale = Sale {
                user_id: self.user_id,
                game_id: self.game_id,
                slot_no: *slot_no,
                amount: *amount,
                price: 0,
            };

            sales.insert(*slot_no, sale);
        }

        Ok(sales)
    }
}

impl Sale {
    fn set_price(&mut self, beverage: &Beverage) {
        self.price = beverage.price();
    }
}

impl Transaction {
    pub fn find_by_id(id: i64, conn: &db::Conn) -> Result<Transaction, DBError> {
        transactions::table
            .filter(transactions::id.eq(id))
            .first::<Transaction>(conn)
    }

    pub fn find_all(
        filter: &TransactionFilter,
        conn: &db::Conn,
    ) -> Result<Vec<Transaction>, ServiceError> {
        let mut query = transactions::table.into_boxed();

        if let Some(game_id) = filter.game_id {
            query = query.filter(transactions::game_id.eq(game_id));
        }

        if let Some(user_id) = filter.user_id {
            query = query.filter(transactions::user_id.eq(user_id));
        }

        let transactions = query
            .order(transactions::created_at.desc())
            .load::<Transaction>(conn)?;
        Ok(transactions)
    }

    /// Get the amount of sales each user has made in a game
    pub fn get_sales_per_user(
        game_id: i64,
        conn: &db::Conn,
    ) -> Result<Vec<UserSales>, ServiceError> {
        use diesel::dsl::sql;

        let sale_count = transactions::table
            .inner_join(users::table)
            .select((
                users::username,
                sql::<diesel::sql_types::BigInt>("CAST (SUM(amount) AS BIGINT)"),
            ))
            .filter(transactions::game_id.eq(game_id))
            .group_by(users::username)
            .load::<UserSales>(conn)?;

        Ok(sale_count)
    }

    /// show the totaol money spend for everyone in a game
    pub fn total_income(game_id: i64, conn: &db::Conn) -> Result<i64, ServiceError> {
        use diesel::dsl::sql;
        let res = transactions::table
            .select(sql::<diesel::sql_types::BigInt>(
                "CAST (SUM(price) AS BIGINT)",
            ))
            .filter(transactions::game_id.eq(game_id))
            .first(conn)?;

        Ok(res)
    }
}

impl SalesCount {
    pub fn initialize_slots(game: &Game, conn: &db::Conn) -> Result<(), DBError> {
        let mut empty_sales: Vec<SalesCount> = Vec::new();
        for slot_no in 0..game.beverage_count {
            empty_sales.push(SalesCount {
                game_id: game.id,
                slot_no,
                sales: 0,
            });
        }

        diesel::insert_into(sales_counts::table)
            .values(&empty_sales)
            .execute(conn)?;

        Ok(())
    }

    /// get salescount for a game while locking the rows during a transaction
    pub(crate) fn find_by_game_for_update(
        game_id: i64,
        conn: &db::Conn,
    ) -> Result<Vec<SalesCount>, DBError> {
        let res = sales_counts::table
            .filter(sales_counts::game_id.eq(game_id))
            .for_update()
            .order_by(sales_counts::slot_no)
            .load::<SalesCount>(conn)?;

        Ok(res)
    }

    pub fn find_by_game(game_id: i64, conn: &db::Conn) -> Result<Vec<SalesCount>, DBError> {
        let res = sales_counts::table
            .filter(sales_counts::game_id.eq(game_id))
            .order_by(sales_counts::slot_no)
            .load::<SalesCount>(conn)?;

        Ok(res)
    }

    fn update(&self, conn: &db::Conn) -> Result<SalesCount, DBError> {
        diesel::update(
            sales_counts::table
                .filter(sales_counts::game_id.eq(self.game_id))
                .filter(sales_counts::slot_no.eq(self.slot_no)),
        )
        .set(sales_counts::sales.eq(self.sales))
        .get_result(conn)
    }

    pub(crate) fn average_sales(sales: &[SalesCount]) -> i64 {
        let mut total: i64 = 0;

        for beverage in sales {
            total += beverage.sales;
        }

        (total as f64 / sales.len() as f64).ceil() as i64
    }

    pub(crate) const fn get_offset(&self, average: i64) -> i64 {
        self.sales - average
    }

    /// Returns a hashmap contianing how much each beverage's sales
    /// differs from the average amount of sales.
    /// This can be used to calculate the price of a beverage
    pub fn get_price_offsets(
        game_id: i64,
        conn: &db::Conn,
    ) -> Result<HashMap<i16, i64>, ServiceError> {
        let sales = SalesCount::find_by_game(game_id, conn)?;
        let average = SalesCount::average_sales(&sales);

        let mut offsets: HashMap<i16, i64> = HashMap::new();

        for slot in sales.iter() {
            let offset = slot.sales - average;
            trace!(
                "beverage({})'s offset in game({}) is {} with {} sales",
                slot.slot_no,
                game_id,
                offset,
                slot.sales
            );
            let conflict = offsets.insert(slot.slot_no, offset).is_some();

            if conflict {
                error!(
                    "unable to calculate offset for slot({}) in game({})",
                    slot.slot_no, game_id
                );
                return Err(ServiceError::InternalServerError);
            }
        }

        Ok(offsets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unroll_sale_to_sales() {
        let mut slots = HashMap::new();
        slots.insert(0, 2);
        slots.insert(1, 1);
        slots.insert(2, 0);
        let sale = NewSale {
            user_id: 1,
            game_id: 1,
            slots,
        };

        let res = sale.unroll().unwrap();
        assert_eq!(res.len(), 3);
    }
}
