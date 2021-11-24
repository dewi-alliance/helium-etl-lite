# Get up and running on Ubuntu 20.04

A quick and dirty guide to get the ETL Lite up and running on AWS's Ubuntu 20.04 AMI.

Once you have your instance up and running:

1. Update apt  
   `sudo apt-get update`
2. Install dependencies  
  `sudo apt-get install docker.io make postresql clang -y`

## Build blockchain-node docker

1. Add yourself to docker group  
  `sudo usermod -aG docker ubuntu`  
  relog into your shell for the changes to take place
2. Download blockchain-node code  
  `git clone git clone https://github.com/helium/blockchain-node.git`
3. Update blockchain-node config to load all json (needed for rewards data)
  `sed -i "/{blockchain,/{N;s/$/\n   {json_store, true},/}" config/dev.config`
4. Build docker container  
  `make docker-build`

## Create database and postgres user
1. Run psql  
  `sudo -u postgres psql`
2. Run SQL commands to create db and user  
  `create database helium_lite;`  
  `create user etlite with encrypted password ‘password’;` Hopefully you choose a better password.  
  `grant all privileges on database helium_lite to etlite;`

## Build ETL Lite
1. Install Rust + Cargo  
  `curl https://sh.rustup.rs -sSf | sh` Then follow instructions
2. Build ETL Lite  
  `git clone https://github.com/dewi-alliance/helium-etl-lite.git`  
  `cd helium-etl-lite`  
  `cargo build --release`

## Running blockchain-node and ETL Lite
1. Start blockcahin-node  
  `cd ../blockchain-node`  
  `make docker-start`
2. Run migrations  
  `cd ../helium-etl-lite`   
  `target/release/helium_etl_lite migrate`   
3. Wait 15-30 min before starting ETL Lite  
  `target/release/helium_etl_lite start`

You should be able to see what's happening in the logs in `log/etl_lite.log`

TODO: service script
