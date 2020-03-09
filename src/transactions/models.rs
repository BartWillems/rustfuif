use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::result::Error as DBError;

use crate::db;
use crate::errors::ServiceError;
use crate::schema::transactions;

#[derive(Debug, Serialize, Queryable, Identifiable, AsChangeset, Clone)]
pub struct Transaction {
    pub id: i64,
    pub user_id: i64,
    pub game_id: i64,
    pub slot_no: i16,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct TransactionFilter {
    pub user_id: Option<i64>,
    pub game_id: Option<i64>,
}

#[derive(Debug, Deserialize, Insertable)]
#[table_name = "transactions"]
pub struct Sale {
    pub user_id: i64,
    pub game_id: i64,
    pub slot_no: i16,
}

#[derive(Debug, Deserialize)]
pub struct NewSale {
    pub user_id: i64,
    pub game_id: i64,
    pub slots: Vec<Slot>,
}

#[derive(Debug, Deserialize)]
pub struct Slot {
    /// determines which beverage is bought
    pub slot_number: i16,
    pub amount: u8,
}

impl NewSale {
    pub fn save(&self, conn: &db::Conn) -> Result<Transaction, ServiceError> {
        let sales = self.unroll()?;

        let transactions = diesel::insert_into(transactions::table)
            .values(sales)
            .get_result::<Transaction>(conn)?;

        Ok(transactions)
    }

    fn unroll(&self) -> Result<Vec<Sale>, ServiceError> {
        let mut sales: Vec<Sale> = Vec::new();
        for slot in &self.slots {
            for _ in 0..slot.amount {
                let sale = Sale {
                    user_id: self.user_id,
                    game_id: self.game_id,
                    slot_no: slot.slot_number,
                };
                sale.validate_slot()?;
                sales.push(sale);
            }
        }

        Ok(sales)
    }
}

impl Sale {
    pub fn validate_slot(&self) -> Result<(), ServiceError> {
        if !(0..8).contains(&self.slot_no) {
            bad_request!("the slot number should be within [0-7]");
        }
        Ok(())
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

        let transactions = query.load::<Transaction>(conn)?;
        Ok(transactions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unroll_sale_to_sales() {
        let sale = NewSale {
            user_id: 1,
            game_id: 1,
            slots: vec![
                Slot {
                    slot_number: 0,
                    amount: 2,
                },
                Slot {
                    slot_number: 1,
                    amount: 1,
                },
                Slot {
                    slot_number: 2,
                    // this should produce no sale record
                    amount: 0,
                },
            ],
        };

        let res = sale.unroll().unwrap();
        assert_eq!(res.len(), 3);
    }

    #[test]
    fn unroll_invalid_sale() {
        let sale = NewSale {
            user_id: 1,
            game_id: 1,
            slots: vec![Slot {
                // 7 is the maximum slot number
                slot_number: 8,
                amount: 2,
            }],
        };

        assert_eq!(sale.unroll().is_err(), true);
    }
}
