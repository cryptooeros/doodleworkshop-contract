#!/bin/bash

#Build Flag
PARAM=$1
####################################    Constants    ##################################################
#depends on mainnet or testnet
#NODE="--node https://rpc.juno.omniflix.co:443"
#CHAIN_ID=juno-1
#DENOM="ujuno"

NODE="--node https://rpc.juno.giansalex.dev:443"
CHAIN_ID=uni-1
DENOM="ujunox"

#not depends
NODECHAIN=" $NODE --chain-id $CHAIN_ID"
TXFLAG=" $NODECHAIN --gas-prices 0.03$DENOM --gas auto --gas-adjustment 1.3"
WALLET="--from workshop"
WASMFILE="artifacts/cw20_escrow.wasm"
TXHASHFILE="uploadtx"
CONTRACTADDRFILE="contractaddr"
WORKSHOPADDR="juno1htjut8n7jv736dhuqnad5mcydk6tf4ydeaan4s"
ACHILLESADDR="juno15fg4zvl8xgj3txslr56ztnyspf3jc7n9j44vhz"

CREWADDR="juno1fjspqgdn4v88rwz9gw8zn4d38fp07cxnrtgw3jtah2j6nymzxgpqqp94xz"
####################################    Functions    ###################################################

OptimizeBuild() {

    echo "================================================="
    echo "Optimize Build Start"
    docker run --rm -v "$(pwd)":/code \
        --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
        --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
        cosmwasm/rust-optimizer:0.12.4
    echo "================================================="
    
}

Upload() {
    echo "================================================="
    echo "Upload $WASMFILE"
    UPLOADTX=$(junod tx wasm store $WASMFILE $WALLET $TXFLAG --output json -y | jq -r '.txhash')
    echo $UPLOADTX
    #save to file
    echo $UPLOADTX > $TXHASHFILE
    echo "writed last transaction hash to $TXHASHFILE"
    echo "================================================="
    
}

GetCode() {
    echo "================================================="
    echo "Get code from txHash written on $TXHASHFILE"
    #read from file
    TXHASH=$(cat $TXHASHFILE)
    echo "read last transaction hash from $TXHASHFILE"
    echo $TXHASH
    QUERYTX="junod query tx $TXHASH $NODECHAIN --output json"
	CODE_ID=$($QUERYTX | jq -r '.logs[0].events[-1].attributes[0].value')
	echo $CODE_ID
    echo "================================================="
    return $CODE_ID
}

GetContractAddress() {
    echo "================================================="
    echo "Get contract address by code"
    GetCode
    CODE_ID=$?
    #QUERYCONTRACT="junod query wasm list-contract-by-code $CODE_ID $NODECHAIN --output json"
    #echo $QUERYCONTRACT
    CONTRACT_ADDR=$(junod query wasm list-contract-by-code $CODE_ID $NODECHAIN --output json | jq -r '.contracts[0]')
    echo $CONTRACT_ADDR > $CONTRACTADDRFILE
    echo $CONTRACT_ADDR
    echo "================================================="
}

GetBalance() {
    GetContractAddress
    CONTRACT_ADDR=$(cat $CONTRACTADDRFILE)
    junod query bank balances $CONTRACT_ADDR $NODECHAIN
}

GetContract() {
    GetContractAddress
    CONTRACT_ADDR=$(cat $CONTRACTADDRFILE)
    junod query wasm contract $CONTRACT_ADDR $NODECHAIN
    junod query wasm contract-state all $CONTRACT_ADDR $NODECHAIN --output json
}

ListCode() {
    echo "================================================="
    echo "Get List of Codes"
    junod query wasm list-code $NODECHAIN --output json
    echo "================================================="
}

Instantiate() {
    GetCode
    CODE_ID=$?
    junod tx wasm instantiate $CODE_ID '{}' --label "WorkShop" $WALLET $TXFLAG -y
}

ListQuery() {
    GetContractAddress
    CONTRACT_ADDR=$(cat $CONTRACTADDRFILE)
    junod query wasm contract-state smart $CONTRACT_ADDR '{"list":{}}' $NODECHAIN
}

DetailsQuery() {
    GetContractAddress
    CONTRACT_ADDR=$(cat $CONTRACTADDRFILE)
    junod query wasm contract-state smart $CONTRACT_ADDR '{"details":{"id":1}}' $NODECHAIN
}

CreateEscrow() {
    GetContractAddress
    CONTRACT_ADDR=$(cat $CONTRACTADDRFILE)
    junod tx wasm execute $CONTRACT_ADDR '{"create":{"id":"$ACHILLESADDR", "arbiter":"$ACHILLESADDR", "recipient":"$ACHILLESADDR"}}' $WALLET $TXFLAG
}


#################################### End of Function ###################################################
# Upload
# QueryTX $?
# echo $? > lasthash
$PARAM
# if [[ $PARAM == "build" ]]; then
#     OptimizeBuild
# fi

#junod tx wasm instantiate 54 '{"name":"CREWStaking", "symbol":"CST", "decimals":6, "validator":"juno1htjut8n7jv736dhuqnad5mcydk6tf4ydeaan4s", "unbonding_period": {"height":"1000", "Time":"1000"}, "exit_tax": 10, "min_withdrawal": 10}' --label "CREWStaking" --from workshop --node https://rpc.juno.giansalex.dev:443 --chain-id uni-1 --gas-prices 0.03ujunox --gas auto --gas-adjustment 1.3 -y

#junod tx wasm instantiate 54 '{"name":"CREWStaking", "symbol":"CST", "decimals":6, "validator":"juno1htjut8n7jv736dhuqnad5mcydk6tf4ydeaan4s", "unbonding_period": {"height":"1000", "Time":"1000"}}' --label "CREWStaking" --from workshop --node https://rpc.juno.giansalex.dev:443 --chain-id uni-1 --gas-prices 0.03ujunox --gas auto --gas-adjustment 1.3 -y
#Step
#Upload
#Instantiate
#GetContract

#junod query wasm contract-state smart juno16q9wqk00prx0nkyqk9r5xzqn5n52fa4vztz6jumv4x76mdqla5hqwwm9ur '{"list":{}}' --node https://rpc.juno.giansalex.dev:443 --chain-id uni-1
