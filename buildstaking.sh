#!/bin/sh

#Build Flag
REBUILD=$1
#Build optimized result
if $REBUILD eq 'TRUE'
  docker run --rm -v "$(pwd)":/code \
    --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
    --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
    cosmwasm/rust-optimizer:0.12.4
fi

#Upload, Instantiate, Query staking contract
NODE="--node https://rpc.juno.giansalex.dev:443"
CHAIN_ID=uni-1
TXFLAG=" $NODE --chain-id $CHAIN_ID --gas-prices 0.03ujunox --gas auto --gas-adjustment 1.3"

junod tx wasm store artifacts/cw20_staking.wasm --from workshop $TXFLAG -y --output json