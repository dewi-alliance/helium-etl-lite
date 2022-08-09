FROM rust:1.62

WORKDIR /usr/src/helium-etl-lite

COPY . .

RUN cargo install --path .

ENTRYPOINT ["/usr/local/cargo/bin/helium_etl_lite"]
