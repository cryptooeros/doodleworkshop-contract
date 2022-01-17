#!/bin/bash

#Build Flag
PARAM=$1
####################################    Constants    ##################################################
#depends on mainnet or testnet
NODE="--node https://rpc.juno.giansalex.dev:443"
CHAIN_ID=uni-1
DENOM="ujunox"

#not depends
NODECHAIN=" $NODE --chain-id $CHAIN_ID"
TXFLAG=" $NODECHAIN --gas-prices 0.03$DENOM --gas auto --gas-adjustment 1.3"
WALLET="--from workshop"
WASMFILE="artifacts/cw20_staking.wasm"
TXHASHFILE="uploadtx"
CONTRACTADDRFILE="contractaddr"
WORKSHOPADDR="juno1htjut8n7jv736dhuqnad5mcydk6tf4ydeaan4s"
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

Instantiate() {
    GetCode
    CODE_ID=$?
    junod tx wasm instantiate $CODE_ID \
    '{"name":"CREWStaking","symbol":"POOD","decimals":6,"initial_balances":[{"address":"$WORKSHOPADDR","amount":"12345678000"}]}' \
    --amount 50000$DENOM --label "Poodlecoin erc20" $WALLET $TXFLAG -y

}

GetContractAddress() {
    echo "================================================="
    echo "Get contract address by code"
    GetCode
    CODE_ID=$?
    #QUERYCONTRACT="junod query wasm list-contract-by-code $CODE_ID $NODECHAIN --output json"
    #echo $QUERYCONTRACT
    CONTRACT_ADDR=$(junod query wasm list-contract-by-code $CODE_ID $NODECHAIN --output json | jq -r '.contracts[0]')
    echo $CONTRACT_ADDR
    echo "================================================="
}

GetBalance() {
    GetContractAddress
    junod query bank balances $? $NODECHAIN
}

GetContract() {
    GetContractAddress
    junod query wasm contract $? $NODECHAIN
}

ListCode() {
    echo "================================================="
    echo "Get List of Codes"
    junod query wasm list-code $NODECHAIN --output json
    echo "================================================="
}



#################################### End of Function ###################################################
# Upload
# QueryTX $?
# echo $? > lasthash
$PARAM
# if [[ $PARAM == "build" ]]; then
#     OptimizeBuild
# fi

# if [[ $PARAM == "upload" ]]; then
#     Upload
# fi

# if [[ $PARAM == "getcode" ]]; then
#     GetCode
# fi

# if [[ $PARAM == "listcode" ]]; then
#     ListCode
# fi

# if [[ $PARAM == "contractaddr" ]]; then
#     GetContractAddress
# fi

# if [[ $PARAM == "instantiate" ]]; then
#     Instantiate
# fi
junod tx wasm instantiate 54 '{"name":"CREWStaking", "symbol":"CST", "decimals":6, "validator":"juno1htjut8n7jv736dhuqnad5mcydk6tf4ydeaan4s", "unbonding_period": {"height":"1000", "Time":"1000"}, "exit_tax": 10, "min_withdrawal": 10}' --label "CREWStaking" --from workshop --node https://rpc.juno.giansalex.dev:443 --chain-id uni-1 --gas-prices 0.03ujunox --gas auto --gas-adjustment 1.3 -y

junod tx wasm instantiate 54 '{"name":"CREWStaking", "symbol":"CST", "decimals":6, "validator":"juno1htjut8n7jv736dhuqnad5mcydk6tf4ydeaan4s"}' --label "CREWStaking" --from workshop --node https://rpc.juno.giansalex.dev:443 --chain-id uni-1 --gas-prices 0.03ujunox --gas auto --gas-adjustment 1.3 -y