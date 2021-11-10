use crate::*;
use tokio_postgres::{Client, Statement};
use std::convert::TryFrom;
use helium_jsonrpc::Transaction;
use tokio_postgres::types::{Json};

pub async fn prepare(client: &Client) -> Result<Statement>{
	let stmt = client.prepare(r#"INSERT INTO transactions (block, hash, type, fields) 
		VALUES ($1, $2, CAST(CAST($3 AS VARCHAR) AS "transaction_type"), $4)"#).await;
	match stmt {
		Ok(s) => Ok(s),
		Err(e) => Err(error::Error::PgError(e)),
	}
}

pub async fn add_transaction(client:  &Client, 
	block: u64, 
	hash: String, 
	r#type: &str, 
	transaction: Transaction) -> Result<Vec<tokio_postgres::Row>> {
	let stmt = prepare(&client).await.unwrap();
	let fields = Json(transaction);

	match client.query(&stmt, &[&i64::try_from(block).unwrap(),
		&hash,
		&r#type,
		&fields]).await {
		Ok(v) => Ok(v),
		Err(e) => {
			println!("{}", e);
			Err(error::Error::PgError(e))
		},
	}
}