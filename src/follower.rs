use crate::*;
use slog::{error, info, o, Logger};
use helium_jsonrpc::{ Client, blocks, blocks::BlockRaw, transactions, Transaction };
use tokio_postgres::{ Client as PgClient };
use std::convert::TryFrom;

pub struct Follower {
	mode: EtlMode,
	pub height: u64,
	first_block: u64,
	client: Client,
	pgclient: PgClient,
	shutdown: triggered::Listener,
	logger: Logger,
}

pub struct Info {
	height: u64,
	first_block: u64,
}

impl Follower {
	pub async fn new(settings: &Settings, pgclient: PgClient, logger: &Logger, shutdown: triggered::Listener) -> Result<Self> {
		let client = Client::new_with_base_url(settings.node_addr.to_string());
		let info = match load_follower_info(&logger, &pgclient).await {
			Ok(i) => i,
			Err(_) => {
				let first = get_first_block(&client, &logger, shutdown.clone()).await.unwrap();
				create_follower_info(&logger, &pgclient, first).await.unwrap();
				Info {
					height: first,
					first_block: first,
				}				
			}
		};

		Ok(Self {
			mode: settings.mode,
			height: info.height,
			first_block: info.first_block,
			client: client,
			pgclient: pgclient,
			shutdown: shutdown,
			logger: logger.new(o!("module" => "follower")),
		})
	}
	pub async fn run(&mut self) {
		loop {
			tokio::select! {
				_ = self.shutdown.clone() => {
					info!(self.logger, "shutting down Follower at height: {}", self.height);
					return
				},
				maybe_current_height = blocks::height(&self.client) => {
					let current_height = match maybe_current_height {
						Ok(ch) => ch,
						Err(e) => {
							error!(self.logger, "Couldn't get height from node: {}", e);
							return
						}
					};
					match current_height {
						h if h > self.height => {
							match self.get_block(&self.logger, self.height+1).await {
								Ok(_) => self.height += 1,
								Err(_) => return,
							}
							self.update_follower_info_height().await.unwrap();
							info!(self.logger, "got block {}", self.height);
						},
						_ => return
					}
				}
			}
		}
	}
	pub async fn get_block(&self, logger: &Logger, height: u64) -> Result<()> {
		match blocks::get_raw(&self.client, &height).await {
			Ok(b) => match self.mode {
				EtlMode::Rewards => self.process_block(&self.logger, b).await,
				_ => panic!("todo"),
			},
			Err(e) => {
				error!(logger, "Couldn't get block {}: {}", height, e);
				return Err(error::Error::Custom(format!("couldn't get block {}: {}", height, e)))
			}
		};
		Ok(())
	}
	pub async fn process_block(&self, logger: &Logger, block: BlockRaw) {
		for txn in block.transactions {
			match txn.r#type.as_str() {
				"rewards_v2" => {
					let rewards = match transactions::get(&self.client, &txn.hash).await {
						Ok(t) => match t {
							Transaction::RewardsV2(rewards) => rewards.rewards,
							_ => {
								error!(logger, "Error getting rewards txn: '{}'", txn.hash);
								return
							}
						},
						Err(e) => {
							error!(logger, "Error getting rewards txn: '{}' {}", txn.hash, e);
							return
						}
					};
					info!(logger, "rewards in block {} with {}", block.height.to_string(), rewards.len());
					for r in rewards {
						match reward::add_reward(&self.pgclient, block.height, block.time, block.hash.to_string(), &r).await {
							Ok(_) => (),
							Err(e) => error!(logger, "Error adding reward {}", e),
						}
					}
				},
				_ => (),
			}
		} 
	}

	pub async fn update_follower_info_first_block(&self) -> Result<Vec<tokio_postgres::Row>>{
		let stmt = self.pgclient.prepare("UPDATE follower_info SET first_block = $1").await.unwrap();
		self.pgclient.query(&stmt, &[&i64::try_from(self.first_block).unwrap()])
			.await
			.map_err(|e| error::Error::PgError(e))
	}

	pub async fn update_follower_info_height(&self) -> Result<Vec<tokio_postgres::Row>>{
		let stmt = self.pgclient.prepare("UPDATE follower_info SET height = $1").await.unwrap();
		self.pgclient.query(&stmt, &[&i64::try_from(self.height).unwrap()])
			.await
			.map_err(|e| error::Error::PgError(e))
	}	
}

pub async fn create_follower_info(logger: &Logger, pgclient: &PgClient, first_block: u64) -> Result<Vec<tokio_postgres::Row>> {
	let stmt = pgclient.prepare("INSERT INTO follower_info (height, first_block) VALUES ($1, $2)").await?;
	info!(logger, "Adding follower info to database");
	pgclient.query(&stmt, &[&i64::try_from(first_block).unwrap(), &i64::try_from(first_block).unwrap()])
		.await
		.map_err(|e| error::Error::PgError(e))
}

pub async fn load_follower_info(logger: &Logger, pgclient: &PgClient) -> Result<Info> {
	let stmt = pgclient.prepare("SELECT height, first_block FROM follower_info").await?;
	let info_rows = pgclient.query(&stmt, &[]).await?;

	match info_rows.len() {
		0 => {
			info!(logger, "no follower_info data");
			Err(error::Error::Custom("no follower info".to_string()))
		}
		_ => {
			let height: i64 = info_rows[0].get(0);
			let first_block: i64 = info_rows[0].get(1);
			Ok(Info {
				height: u64::try_from(height).unwrap(),
				first_block: u64::try_from(first_block).unwrap(),
			})

		}
	}
}

pub async fn get_first_block(client: &Client, logger: &Logger, shutdown: triggered::Listener) -> Result<u64> {
	info!(logger, "Scanning blocks by epoch to get first block on node.");
	let mut height = blocks::height(&client).await?;
	let mut last_safe_height = height;
	let mut in_last_epoch = false;

	loop {
		tokio::select! {
			_ = shutdown.clone() => {
				info!(logger, "abandoning get_first_height at height: {}", last_safe_height);
				return Ok(last_safe_height)
			},
			blockraw = blocks::get_raw(&client, &height) => {
				let block = match blockraw {
					Ok(b) => b,
					Err(_) if in_last_epoch => return Ok(last_safe_height),
					Err(_) => {
						in_last_epoch = true;
						height = last_safe_height - 1;
						match blocks::get_raw(&client, &height).await {
							Ok(b) => b,
							Err(e) => panic!("Can't get last height, stuck on block {}: {}", height, e),
						}
					}
				};
				last_safe_height = height;
				for txn in block.transactions {
					match txn.r#type.as_str() {
						"rewards_v2" => {
							info!(&logger, "Getting start_epoch from block {}", height);
							match transactions::get(&client, &txn.hash).await.unwrap() {
								Transaction::RewardsV2(rewards) => height = rewards.start_epoch,
								_ => ()
							}
						},
						_ => (),
					};
				}
				height -= 1;	
			}
		}
	}
}