use crate::*;
use slog::{error, info, Logger};
use helium_jsonrpc::{ Client, blocks, blocks::BlockRaw, transactions, Transaction };
use tokio_postgres::{ Transaction as PgTransaction };
use std::convert::TryFrom;

pub struct BlockProcessor<'a> {
    mode: EtlMode,
    height: u64,
    client: &'a Client,
    pgtran: PgTransaction<'a>,
    logger: &'a Logger,
    filters: &'a filter::Filters,
}

impl<'a> BlockProcessor<'a> {
    pub fn new(mode: EtlMode, height: u64, client: &'a Client, pgtran: PgTransaction<'a>, logger: &'a Logger, filters: &'a filter::Filters) -> Self {
        BlockProcessor{
            mode,
            height,
            client,
            pgtran,
            logger,
            filters
        }
    }

    pub async fn process(mut self) -> Result<()> {
        match blocks::get_raw(&self.client, &self.height).await {
            Ok(b) => self.load_block(b).await?,
            Err(e) => {
                error!(self.logger, "Couldn't get block {}: {}", self.height, e);
                return Err(error::Error::Custom(format!("couldn't get block {}: {}", self.height, e)))
            },
        }

        info!(self.logger, "got block {}", self.height);
        self.height += 1;

        match self.update_follower_info_height().await {
            Ok(_) => {},
            Err(e) => return Err(e),
        }

        match self.pgtran.commit().await {
            Ok(_) => Ok(()),
            Err(e) => Err(error::Error::custom(e.to_string())),
        }
    }

    async fn load_block(&self, block: BlockRaw) -> Result<()> {
        match self.mode {
            EtlMode::Full => info!(self.logger, "Loading txns in block {}", block.height),
            _ => (),
          }
          for txn in &block.transactions {
            match txn.r#type.as_str() {
              "rewards_v2" => {
                let rewards = match transactions::get(&self.client, &txn.hash).await {
                  Ok(t) => match t {
                    Transaction::RewardsV2(rewards) => rewards.rewards,
                    _ => {
                      error!(self.logger, "Error getting rewards txn: '{}'", txn.hash);
                      return Err(error::Error::Custom(format!("Error getting rewards txn: '{}'", txn.hash)))
                    }
                  },
                  Err(e) => {
                    error!(self.logger, "Error getting rewards txn: '{}' {}", txn.hash, e);
                    return Err(error::Error::Custom(format!("Error getting rewards txn: '{}' {}", txn.hash, e)))
                  }
                };
                info!(self.logger, "rewards in block {} with {}", block.height.to_string(), rewards.len());
                'rloop: for r in rewards {
                  match self.mode {
                    EtlMode::Rewards | EtlMode::Full => {
                      match reward::add_reward(&self.pgtran, block.height, block.time, block.hash.to_string(), &r).await {
                        Ok(_) => (),
                        Err(e) => error!(self.logger, "Error adding reward {}", e),
                      }
                    },
                    EtlMode::Filters => {
                      match r.account {
                        Some(ref a) => {
                          for filter_account in &self.filters.accounts {
                            match &a {
                              f if f == &filter_account => {
                                info!(self.logger, "loading reward for account: {} -> {}", filter_account, r.r#type);
                                match reward::add_reward(&self.pgtran, block.height, block.time, block.hash.to_string(), &r).await {
                                  Ok(_) => (),
                                  Err(e) => error!(self.logger, "Error adding reward {}", e),
                                }                             
                                continue 'rloop;
                              },
                              _ => (),
                            }
                          }
                        },
                        None => (),
                      }
                      match r.gateway {
                        Some(ref g) => {
                          for filter_gateway in &self.filters.gateways {
                            match &g {
                              f if f == &filter_gateway => {
                                info!(self.logger, "loading reward for gateway: {} -> {}", filter_gateway, r.r#type);
                                match reward::add_reward(&self.pgtran, block.height, block.time, block.hash.to_string(), &r).await {
                                  Ok(_) => (),
                                  Err(e) => error!(self.logger, "Error adding reward {}", e),
                                }                                                           
                                continue 'rloop;
                              },
                              _ => (),
                            }
                          }
                        },
                        None => (),                     
                      }
                    },
                  }
                }
              },
              _ => (),
            }
            match self.mode {
              EtlMode::Full => {
      
                let transaction = match transactions::get(&self.client, &txn.hash).await {
                  Ok(t) => t,
                  Err(e) => {
                    error!(self.logger, "Error getting transaction: [{}] {} {}",  txn.r#type, txn.hash, e);
                    return Err(error::Error::Custom(format!("Error getting transaction: [{}] {} {}",  txn.r#type, txn.hash, e)))
                  }
                };
                match transaction::add_transaction(&self.pgtran, block.height, txn.hash.to_string(), txn.r#type.as_str(), transaction).await {
                  Ok(_) => (),
                  Err(e) => error!(self.logger, "Error adding transaction: {}. {}", txn.hash, e),
                }
              },
              _ => (),              
            }
          }
          Ok(()) 
    }

    async fn update_follower_info_height(&self) -> Result<Vec<tokio_postgres::Row>>{
        let stmt = self.pgtran.prepare("UPDATE follower_info SET height = $1").await.unwrap();
        self.pgtran.query(&stmt, &[&i64::try_from(self.height).unwrap()])
          .await
          .map_err(|e| error::Error::PgError(e))
      }
}