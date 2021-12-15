use crate::*;
use tokio_postgres::{Client, Statement};

pub struct Filters {
	pub accounts: Vec<String>,
	pub gateways: Vec<String>,
}

pub async fn prepare(client: &Client) -> Result<Statement>{
	let stmt = client.prepare("SELECT type::varchar(255), value FROM filters").await;
	match stmt {
		Ok(s) => Ok(s),
		Err(e) => Err(error::Error::PgError(e)),
	}
}

pub async fn get(client: &Client) -> Result<Filters>{
	let stmt = prepare(&client).await.unwrap();
	let filter_rows = match client.query(&stmt, &[]).await {
		Ok(rows) => rows,
		Err(e) => {
			println!("{}", e);
			return Err(error::Error::PgError(e))			
		},
	};
	let accounts: Vec<String> = filter_rows
		.iter()
		.filter(|f| f.get::<_, String>("type") == "account")
		.map(|f| f.get::<_, String>("value"))
		.collect();

	let gateways: Vec<String> = filter_rows
		.iter()
		.filter(|f| f.get::<_, String>("type") == "gateway")
		.map(|f| f.get::<_, String>("value"))
		.collect();

	Ok(Filters{accounts: accounts, gateways: gateways})
}