use crate::*;
use slog::{error, info, o, Logger};
use helium_jsonrpc::{ Client, blocks, blocks::BlockRaw, transactions, Transaction };
use tokio_postgres::{ Client as PgClient };
use std::convert::TryFrom;
use crate::block_processor::BlockProcessor;

pub struct Follower {
  mode: EtlMode,
  pub height: u64,
  first_block: u64,
  client: Client,
  pgclient: PgClient,
  shutdown: triggered::Listener,
  logger: Logger,
  filters: filter::Filters,
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
        let first = match settings.backfill {
          true => get_first_block(&client, &logger, shutdown.clone()).await.unwrap(),
          false => blocks::height(&client).await?
        };
        
        create_follower_info(&logger, &pgclient, first).await.unwrap();
        Info {
          height: first-1,
          first_block: first,
        }       
      }
    };

    let logger = match settings.mode {
      EtlMode::Rewards => logger.new(o!("module" => "RewardsMode")),
      EtlMode::Full => logger.new(o!("module" => "FullMode")),
      EtlMode::Filters => logger.new(o!("module" => "FiltersMode")),
    };

    let filters = match settings.mode {
      EtlMode::Filters => {
        match filter::get(&pgclient).await {
          Ok(f) => f,
          Err(e) => panic!("problem getting filters: {}", e),
        }
      },
      _ => filter::Filters{ accounts: vec!(), gateways: vec!() },
    };
    Ok(Self {
      mode: settings.mode,
      height: info.height,
      first_block: info.first_block,
      client: client,
      pgclient: pgclient,
      shutdown: shutdown,
      logger: logger,
      filters: filters,
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
            h if h > self.height => match self.start_block_processing().await {
              Ok(_) => (),
              Err(_) => {},
            },
            _ => return
          }
        }
      }
    }
  }
  pub async fn start_block_processing(&mut self) -> Result<()> {
    let pgtran = match self.pgclient.build_transaction().start().await {
      Ok(t) => t,
      Err(e) => {
        error!(self.logger, "Couldn't start database transaction: {}", e);
        return Err(error::Error::custom(e.to_string()));
      }
    };

    let mut block_processor = BlockProcessor::new(self.mode, self.height, &self.client, pgtran, &self.logger, &self.filters);
    block_processor.process().await?;

    self.height += 1;

    Ok(())
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
        for txn in block.transactions {
          match txn.r#type.as_str() {
            "rewards_v2" => {
              info!(&logger, "Getting start_epoch from block {}", height);
              match transactions::get(&client, &txn.hash).await {
                Ok(t) => {
                  match t {
                    Transaction::RewardsV2(rewards) => height = rewards.start_epoch,
                    _ => ()
                  }
                }
                Err(e) => {
                  error!(logger, "Error getting rewards txn: {}", e);
                  return Ok(last_safe_height);
                }
              }
            },
            _ => (),
          };
        }
        last_safe_height = height;        
        height -= 1;  
      }
    }
  }
}