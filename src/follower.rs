use crate::*;
use slog::{error, info, o, warn, Logger};
use helium_jsonrpc::{ Client, blocks, blocks::Block, blocks::BlockRaw, transactions, transactions::Transaction, error };
use tokio_postgres::{Client as PgClient, NoTls, Error, Statement};



pub struct Follower {
	mode: EtlMode,
	height: u64,
	client: Client,
	pclient: PgClient,
}

// first block: 910077
//919595 rewards -> 919623

impl Follower {
	pub async fn new(settings: &Settings, pclient: PgClient) -> Result<Self> {
		// height = self.find_first
		Ok(Self {
			mode: settings.mode,
			height: 947621,
			client: Client::new_with_base_url(settings.node_addr.to_string()),
			pclient: pclient,
		})
	}
	pub async fn run(&mut self, shutdown: triggered::Listener, logger: &Logger) {
		let logger = logger.new(o!("module" => "follower"));
		let first = self.get_first_block(&logger, shutdown.clone()).await.unwrap();
		info!(logger, "First block: {}", first);
		loop {
			tokio::select! {
				_ = shutdown.clone() => {
					info!(logger, "shutting down Follower at height: {}", self.height);
					return
				},
				current_height = blocks::height(&self.client) => {
					match current_height.unwrap() - self.height {
						0 => {
							info!(logger, "height diff is 0.");
							return
						},
						_ => {
							self.height += 1;
							self.get_block(&logger, self.height).await;
							info!(logger, "got block {}", self.height);
						}
					}
				}
			}
		}
	}
	pub async fn get_block(&self, logger: &Logger, height: u64) {
		match blocks::get_raw(&self.client, &height).await {
			Ok(b) => match self.mode {
				EtlMode::Rewards => self.process_block(&logger, b).await,
				_ => panic!("todo"),
			},
			Err(e) => error!(logger, "Couldn't get block {}: {}", height, e),
		}
		
	}
	pub async fn process_block(&self, logger: &Logger, block: BlockRaw) {
		for txn in block.transactions {
			match txn.r#type.as_str() {
				"rewards_v2" => {
					let rewards = match transactions::get(&self.client, &txn.hash).await {
						Ok(t) => match t {
							Transaction::RewardsV2 { rewards, .. } => rewards,
							_ => {
								error!(logger, "Error getting rewards txn: '{}'", txn.hash);
								return
							}
						},
						Err(e) => {
							error!(logger, "Error getting rewards txn: '{}'", txn.hash);
							return
						}
					};
					info!(logger, "rewards in block {} with {}", block.height.to_string(), rewards.len());
					for r in rewards {
						let rr = reward::add_reward(&self.pclient, block.height, block.time, block.hash.to_string(), &r).await;
						match rr {
							Ok(rr) => (),
							Err(e) => error!(logger, "Error adding reward {}", e),
						}
					}
				},
				_ => (),
			}
		} 
	}

	pub async fn get_first_block(&self, logger: &Logger, shutdown: triggered::Listener) -> Result<u64> {
		info!(logger, "Scanning blocks by epoch to get first block on node.");
		let mut height = blocks::height(&self.client).await.unwrap();
		let mut last_safe_height = height;
		let mut in_last_epoch = false;

		loop {
			tokio::select! {
				_ = shutdown.clone() => {
					info!(logger, "shutting down Follower at height: {}", self.height);
					return Ok(last_safe_height)
				},
				blockraw = blocks::get_raw(&self.client, &height) => {
					let block = match blockraw {
						Ok(b) => b,
						Err(_) if in_last_epoch => return Ok(last_safe_height),
						Err(_) => {
							in_last_epoch = true;
							height = last_safe_height - 1;
							match blocks::get_raw(&self.client, &height).await {
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
								match transactions::get(&self.client, &txn.hash).await.unwrap() {
									Transaction::RewardsV2 { start_epoch, .. } => height = start_epoch,
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
}