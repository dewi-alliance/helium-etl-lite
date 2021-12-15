# Helium Blockchain ETL Lite

A "Light" ETL for the Helium blockchain. A lower resource-intensive option to Helium's [full ETL](https://github.com/helium/blockchain-etl/)

This tool is best suited for collecting new data on the blockchain as it's happening versus historical data. You should probably use the full ETL if you need historical data.

ETL Lite relies on Helium's [blockchain-node]() to extract its data.

**NOTE**: ETL Lite will only load data as old as the height of the snapshot loaded in blockchain-node.  
For more historical data please use Helium's [Full ETL](https://github.com/helium/blockchain-etl/).

## Get Started

For a more in-depth example, see the [quick start guide](https://github.com/dewi-alliance/helium-etl-lite/blob/main/docs/quick-start.md) in `docs`

1. Follow [these steps](https://github.com/helium/blockchain-node#developer-usage) to build and run Helium's blockchain-node. 
**NOTE** You must change `{store_json, false}` to `{store_json, true}` in blockchain_node's sys.config file. Run this command before `make release`

  `sed -i "/{blockchain,/{N;s/$/\n   {store_json, true},/}" config/dev.config`
  
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

## Settings
Settings are found in the `settings.toml` file in the `config` directory.

`log_dir` : location to create log files. Default is `log` in the parent directory.  
`mode`    : Choose which mode to run ETL Lite in. Currently (phase 1) only `rewards` is supported.  
`node_addr` : Ip address for blockchain-node.  
`database_url` : Url to postgresql server. 

## Mode Options
ETL Lite is currently in `Phase 3`

*For Full mode: state_channel_close_v1 transactions are currently not supported due to [this](https://github.com/dewi-alliance/helium-jsonrpc-rs/issues/8) issue.*


| Phase     | Modes Available				 |
| --------- | ---------------------- |
| 1         | Rewards   						 |
| 2         | Rewards, Full 				 |
| 3         | Rewards, Full, Filters | 

Rewards: Load all rewards only  
Full: Load all transactions including rewards  
Filters: Filter what is loaded by either gateway or account

### Filters
Filters must be added to the `filters` table. Rewards can be filtered by `account` or `gateway`.

example:

`INSERT INTO filters (type, value) VALUES ('account', '13oNZxczcP2urLzQTGQVFpezg4C3EADqjcTmDDH5yrpAzi389HL')`

## Database schemas
                   Table "public.rewards"
|     Column      |  Type  | Collation | Nullable | Default |
|-----------------|--------|-----------|----------|---------|
|block            | bigint |           | not null |
|transaction_hash | text   |           | not null |
|time             | bigint |           | not null |
|account          | text   |           | not null |
|gateway          | text   |           | not null |
|amount           | bigint |           | not null |
|type             | text   |           | not null |

             Table "public.follower_info"
|  Column    |  Type  | Collation | Nullable | Default |
|------------|--------|-----------|----------|---------|
|height      | bigint |           | not null |
|first_block | bigint |           | not null |

                Table "public.transactions"
| Column |       Type       | Collation | Nullable | Default |
|--------|------------------|-----------|----------|---------|
| block  | bigint           |           | not null |
| hash   | text             |           | not null |
| type   | transaction_type |           | not null |
| fields | jsonb            |           | not null |
Indexes:
    "transactions_pkey" PRIMARY KEY, btree (hash)
    "transaction_block_idx" btree (block)
    "transaction_type_idx" btree (type)

                    Table "public.filters"
| Column |    Type     | Collation | Nullable | Default |
|--------|-------------|-----------|----------|---------|
| type   | filter_type |           | not null |
| value  | text        |           | not null |

## Rewards Data Note
Because of the way blockchain-node stores rewards info, the first ~300 blocks after the snapshot height won't incldue `gateway` or `type` information for specific rewards. All rewards with `type = 'rewards_v2'` are the total rewards paid to that account vs individual rewards that you will see being loaded into the rewards db after the first ~300 blocks after the snapshot height of the node.

*Please note*: anytime you see `1Wh4bh` as a value, this is the hash for null. You will see this in rewards of type `securities` and `rewards_v2` in the gateway field and for type `overages` in the account field. 