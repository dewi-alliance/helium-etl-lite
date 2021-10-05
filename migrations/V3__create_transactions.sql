CREATE TYPE transaction_type as ENUM (
				'coinbase_v1',
				'security_coinbase_v1',
				'oui_v1',
				'gen_gateway_v1',
				'routing_v1',
				'payment_v1',
				'security_exchange_v1',
				'consensus_group_v1',
				'add_gateway_v1',
				'assert_location_v1',
				'create_htlc_v1',
				'redeem_htlc_v1',
				'poc_request_v1',
				'poc_receipts_v1',
				'vars_v1',
				'rewards_v1',
				'token_burn_v1',
				'dc_coinbase_v1',
				'token_burn_exchange_rate_v1',
				'payment_v2',
				'state_channel_open_v1',
				'state_channel_close_v1',
				'price_oracle_v1',
				'transfer_hotspot_v1',
				'rewards_v2',
				'assert_location_v2',
				'gen_validator_v1',
				'stake_validator_v1',
				'unstake_validator_v1',
				'validator_heartbeat_v1',
				'transfer_validator_stake_v1',
				'gen_price_oracle_v1',
				'consensus_group_failure_v1'
);

CREATE TABLE transactions (
       block BIGINT NOT NULL,
       hash TEXT NOT NULL,
       type transaction_type NOT NULL,
       fields jsonb NOT NULL,

       PRIMARY KEY (hash)
);

CREATE INDEX transaction_type_idx on transactions(type);
CREATE INDEX transaction_block_idx on transactions(block);