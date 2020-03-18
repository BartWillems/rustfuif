use std::collections::HashMap;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::result::Error as DBError;

use crate::db;
use crate::errors::ServiceError;
use crate::schema::transactions;

pub const MIN_SLOT_NO: i16 = 0;
pub const MAX_SLOT_NO: i16 = 7;

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
    pub slots: HashMap<i16, u8>,
}

impl NewSale {
    pub fn save(&self, conn: &db::Conn) -> Result<Vec<Transaction>, ServiceError> {
        let sales = self.unroll()?;

        let transactions = diesel::insert_into(transactions::table)
            .values(sales)
            .get_results::<Transaction>(conn)?;

        Ok(transactions)
    }

    /// turn the map of slots to a list of sales
    fn unroll(&self) -> Result<Vec<Sale>, ServiceError> {
        let mut sales: Vec<Sale> = Vec::new();

        for (slot_no, amount) in &self.slots {
            if slot_no < &MIN_SLOT_NO || slot_no > &MAX_SLOT_NO {
                bad_request!("the slot number should be within [0-7]");
            }
            for _ in 0..*amount {
                let sale = Sale {
                    user_id: self.user_id,
                    game_id: self.game_id,
                    slot_no: *slot_no,
                };
                sales.push(sale);
            }
        }

        Ok(sales)
    }
}

impl Sale {
    pub fn validate_slot(&self) -> Result<(), ServiceError> {
        if !(MIN_SLOT_NO..MAX_SLOT_NO + 1).contains(&self.slot_no) {
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

/// contains how many sales have been made for a given slot
#[derive(Serialize, Queryable, Clone, Copy, Debug, Default)]
pub struct SlotSale {
    pub slot_no: i16,
    pub sales: i64,
}

impl SlotSale {
    pub fn get_sales(game_id: i64, conn: &db::Conn) -> Result<Vec<SlotSale>, ServiceError> {
        use diesel::dsl::sql;

        let sales: Vec<SlotSale> = transactions::table
            .select((
                transactions::slot_no,
                sql::<diesel::sql_types::BigInt>("count(*)"),
            ))
            .filter(transactions::game_id.eq(game_id))
            .group_by(transactions::slot_no)
            .order(transactions::slot_no)
            .load::<SlotSale>(conn)?;

        let sales = SlotSale::fill_gaps(sales);
        Ok(sales)
    }

    /// takes a vector of slotsales and fills in the missing slots
    fn fill_gaps(sales: Vec<SlotSale>) -> Vec<SlotSale> {
        let mut sales_counter: i16 = 0;
        let mut res: Vec<SlotSale> = Vec::with_capacity(MAX_SLOT_NO as usize);
        for i in MIN_SLOT_NO..MAX_SLOT_NO + 1 {
            let sale: SlotSale = sales
                .get(sales_counter as usize)
                .copied()
                .unwrap_or_default();

            if sale.slot_no == i {
                res.push(sale);
                sales_counter += 1;
            } else {
                res.push(SlotSale {
                    slot_no: i,
                    sales: 0,
                });
            }
        }

        res
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

    #[test]
    fn unroll_sale_with_higher_than_max_slot_no() {
        let mut slots = HashMap::new();
        slots.insert(8, 2);
        let sale = NewSale {
            user_id: 1,
            game_id: 1,
            slots,
        };

        assert_eq!(sale.unroll().is_err(), true);
    }

    #[test]
    fn unroll_sale_with_lower_than_minimum_slot_no() {
        let mut slots = HashMap::new();
        slots.insert(-1, 2);
        let sale = NewSale {
            user_id: 1,
            game_id: 1,
            slots,
        };

        assert_eq!(sale.unroll().is_err(), true);
    }

    #[test]
    fn fill_gaps_with_empty_slot_sales() {
        let mut slot_sales = Vec::new();
        slot_sales.push(SlotSale {
            slot_no: 0,
            sales: 5,
        });

        slot_sales.push(SlotSale {
            slot_no: 7,
            sales: 1,
        });

        slot_sales = SlotSale::fill_gaps(slot_sales);

        assert_eq!(slot_sales.len(), (MAX_SLOT_NO + 1) as usize);

        for i in MIN_SLOT_NO..MAX_SLOT_NO + 1 {
            assert_eq!(slot_sales.get(i as usize).unwrap().slot_no, i);
        }
    }
}
