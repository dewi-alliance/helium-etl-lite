use crate::*;
use tokio_postgres::{Client, Statement};
use helium_jsonrpc::transactions;
use std::convert::TryFrom;


pub struct Reward {
	pub block: i64,
	pub transaction_hash: String,
	pub time : i64,
	pub account: String,
	pub gateway: String,
	pub amount: i64,
}

pub async fn prepare(client: &Client) -> Result<Statement>{
	let stmt = client.prepare("INSERT INTO rewards (block, transaction_hash, time, account, gateway, amount, type) 
		VALUES ($1, $2, $3, $4, $5, $6, $7)").await;
	match stmt {
		Ok(s) => Ok(s),
		Err(e) => Err(error::Error::PgError(e)),
	}
}

pub async fn add_reward(client: &Client, 
	block: u64, 
	time: u64, 
	hash: String, 
	reward: &transactions::Reward) -> Result<Vec<tokio_postgres::Row>> {
	let stmt = prepare(&client).await.unwrap();
	let gateway: &String;
	let default: &String = &String::from("1Wh4bh");
	// if reward.gateway.is_none() {gateway = String::from("1Wh4bh");}
	gateway = match &reward.gateway {
		Some(g) => g,
		None => default,
	};
	// for overages
	let account = match &reward.account {
		Some(a) => a,
		None => default,
	};

	// let maybe_gateway = reward.gateway.get_or_insert_with(|| String::from("1Wh4bh"));
	// let gateway: &String = &*(maybe_gateway);
	match client.query(&stmt, &[&i64::try_from(block).unwrap(), 
			&hash, 
			&i64::try_from(time).unwrap(), 
			&account, 
			&gateway,
			&i64::try_from(reward.amount).unwrap(),
			&reward.r#type]).await {
		Ok(v) => Ok(v),
		Err(e) => {
			println!("{}", e);
			Err(error::Error::PgError(e))
		},
	}		
}

