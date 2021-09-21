create table rewards (
       block bigint not null,
       transaction_hash text not null,
       time bigint not null,
       account text not null,
       gateway text not null,
       amount bigint not null,
       type text not null
);

create index on rewards (block);