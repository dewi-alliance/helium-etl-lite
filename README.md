# Helium Blockchain ETL Lite
A "Light" ETL for the Helium blockchain. A lower resource-intensive option to Helium's [full ETL](https://github.com/helium/blockchain-etl/)

This tool is best suited for collecting new data on the blockchain as it's happening versus historical data. You should probably use the full ETL if you need historical data.

ETL Lite relies on Helium's [blockchain-node]() to extract its data.

**NOTE**: ETL Lite will only load data as old as the snapshot loaded in blockchain-node.

## Get Started

1. Follow [these steps](https://github.com/helium/blockchain-node#developer-usage) to build and run Helium's blockchain-node. 
2. Install cargo + rust 
   
   `curl https://sh.rustup.rs -sSf | sh`

3. Install postgresql
   
   `sudo apt get install postgresql`
   *or for MacOS*
   `brew install postgresql`

4. Clone this repo

   `git clone https://github.com/dewi-alliance/helium-etl-lite.git`

5. Build from source

   `cargo build --release`
   
6. Update `settings.toml`. `node_addr` is your blockchain-node and `database_url` should be your postgres db
7. Run `target/release/helium_etl_lite migrate` to run migrations then `target/release/helium_etl_lite start`

## Mode Options
ETL Lite is currently in `Phase 1` 

| Phase     | Modes      						 |
| --------- | ---------------------- |
| 1         | Rewards   						 |
| 2         | Rewards, Full 				 |
| 3         | Rewards, Full, Filters |

## Settings
Settings are found in the `settings.toml` file in the `config` directory.

`log_dir` : location to create log files. Default is `log` in the parent directory.
`mode`    : Choose which mode to run ETL Lite in. Currently (phase 1) `rewards` is supported.
`node_addr` : Ip address for blockchain-node.
`database_url` : Url to postgresql server.

## Database schemas
                   Table "public.rewards"
      Column      |  Type  | Collation | Nullable | Default
------------------+--------+-----------+----------+---------
 block            | bigint |           | not null |
 transaction_hash | text   |           | not null |
 time             | bigint |           | not null |
 account          | text   |           | not null |
 gateway          | text   |           | not null |
 amount           | bigint |           | not null |
 type             | text   |           | not null |