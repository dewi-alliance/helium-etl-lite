CREATE TYPE filter_type as ENUM (
        'gateway',
        'account'
);

CREATE TABLE filters (
       type filter_type NOT NULL,
       value TEXT NOT NULL,

       PRIMARY KEY (value)
);