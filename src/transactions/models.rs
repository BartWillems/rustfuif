use std::collections::HashMap;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::result::Error as DBError;
use sqlx::{Pool, Postgres};

use crate::db;
use crate::errors::ServiceError;
use crate::games::{Beverage, Game};
use crate::schema::{sales_counts, transactions, users};

#[derive(Debug, Serialize, Queryable, Identifiable, AsChangeset, Clone)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct TransactionFilter {
    pub user_id: Option<i64>,
    pub game_id: Option<i64>,
}

#[derive(Debug, Deserialize, Insertable, Clone, Copy)]
#[serde(rename_all = "camelCase")]
#[table_name = "transactions"]
pub struct Sale {
    pub user_id: i64,
    pub game_id: i64,
    pub slot_no: i16,
    pub amount: i32,
    pub price: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewSale {
    pub user_id: i64,
    pub game_id: i64,
    pub slots: HashMap<i16, i32>,
}

/// contains how many sales have been made for a given slot
#[derive(Serialize, Queryable, Clone, Copy, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct SlotSale {
    pub slot_no: i16,
    pub sales: i64,
}

#[derive(Debug, Serialize, Queryable)]
pub struct UserSales {
    pub username: String,
    pub sales: i64,
}

#[derive(Debug, Serialize, Deserialize, Insertable, Queryable)]
#[serde(rename_all = "camelCase")]
pub struct SalesCount {
    pub game_id: i64,
    pub slot_no: i16,
    pub sales: i64,
}

impl NewSale {
    #[tracing::instrument(name = "transaction::purchase")]
    pub async fn save(&self, db: &Pool<Postgres>) -> Result<Vec<Transaction>, ServiceError> {
        // NEW SALES ORDER
            // 1. Fetch beverage configs FOR UPDATE
            // 2. Fetch current sales_counts FOR UPDATE
            // 3. Calculate the prices for each beverage in the new sale
            // 4. update sales_counts
            // 5. insert in transactions with the current count
        let tx = db.begin().await?;
        let game = Game::find_by_id(self.game_id, db).await?;

        for slot_no in self.slots.keys() {
            if slot_no > &game.beverage_count || slot_no < &0 {
                bad_request!("a beverage slot exceeds the maximum configured beverage slots");
            }
        }

        let mut sales: HashMap<i16, Sale> = self.unroll();
        let keys: Vec<i16> = sales.keys().copied().collect();

        // 1
        let beverages = sqlx::query_as!(
            Beverage, 
            "SELECT * FROM beverages WHERE user_id = $1 AND game_id = $2 and slot_no = any($3) FOR UPDATE", 
            self.user_id, self.game_id, &keys)
            .fetch_all(db)
            .await?;
        
        // 2
        let mut sales_counts = SalesCount::find_by_game_for_update(self.game_id, db).await?;

        // 3
        for (_, sale) in sales.iter_mut() {
            let mut beverage_config: Option<&Beverage> = None;
            for beverage in &beverages {
                if beverage.slot_no == sale.slot_no {
                    beverage_config = Some(beverage);
                    break;
                }
            }
            match beverage_config {
                None => {
                    error!("a sale was attempted without a pre-existing beverage config");
                    bad_request!("unable to create purchase for beverage without a config");
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
                sale_count.update(db).await?;
            }
        }

        // 5
        let mut transactions: Vec<Transaction> = Vec::new();

        for sale in sales.values() {
            let transaction = sqlx::query_as!(
                Transaction,
                "INSERT INTO transactions (user_id, game_id, slot_no, amount, price) VALUES ($1, $2, $3, $4, $5) RETURNING *",
                sale.user_id, sale.game_id, sale.slot_no, sale.amount, sale.price
            ).fetch_one(db).await?;
            transactions.push(transaction);
        }

        tx.commit().await?;

        Ok(transactions)
    }

    /// turn the map of slots to a map of sales with their slot no as key
    fn unroll(&self) -> HashMap<i16, Sale> {
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

        sales
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

    #[tracing::instrument(skip(conn), name = "Transaction::find_all")]
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
    #[tracing::instrument(skip(conn))]
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

    /// show the total money spend for everyone in a game
    #[tracing::instrument(skip(conn))]
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
    /// Create the empty beverage sale count rows
    /// Should be called when initializing the game
    pub async fn initialize_slots(game: &Game, db: &Pool<Postgres>) -> Result<(), sqlx::Error> {
        // Inserting multiple values isn't supported yet sadly: https://github.com/launchbadge/sqlx/issues/294
        for slot_no in 0..game.beverage_count {
            sqlx::query!(
                "INSERT INTO sales_counts (game_id, slot_no, sales) VALUES ($1, $2, $3)",
                game.id,
                slot_no,
                0
            )
            .execute(db)
            .await?;
        }

        Ok(())
    }

    /// get salescount for a game while locking the rows during a transaction
    #[tracing::instrument(name = "salescount::find_by_game_for_update")]
    pub(crate) async fn find_by_game_for_update(
        game_id: i64,
        db: &Pool<Postgres>,
    ) -> Result<Vec<SalesCount>, sqlx::Error> {

        let res = sqlx::query_as!(SalesCount, "SELECT * FROM sales_counts WHERE game_id = $1 ORDER BY slot_no FOR UPDATE", game_id).fetch_all(db).await?;

        Ok(res)
    }

    #[tracing::instrument(skip(conn))]
    pub fn find_by_game(game_id: i64, conn: &db::Conn) -> Result<Vec<SalesCount>, DBError> {
        let res = sales_counts::table
            .filter(sales_counts::game_id.eq(game_id))
            .order_by(sales_counts::slot_no)
            .load::<SalesCount>(conn)?;

        Ok(res)
    }

    #[tracing::instrument(name = "SalesCount::update")]
    async fn update(&self, db: &Pool<Postgres>) -> Result<SalesCount, sqlx::Error> {
        sqlx::query_as!(SalesCount, "UPDATE sales_counts SET sales = $1 WHERE game_id = $2 AND slot_no = $3 RETURNING *", self.sales, self.game_id, self.slot_no).fetch_one(db).await
    }

    /// Returns the aveage sales for a game
    pub(crate) fn average_sales(sales: &[SalesCount]) -> i64 {
        let mut total: i64 = 0;

        for beverage in sales {
            total += beverage.sales;
        }

        (total as f64 / sales.len() as f64).ceil() as i64
    }

    /// Returns the distance between the current sales and the average
    pub(crate) const fn get_offset(&self, average: i64) -> i64 {
        self.sales - average
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

        let res = sale.unroll();
        assert_eq!(res.len(), 3);
    }
}
