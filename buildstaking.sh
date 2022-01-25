#!/bin/bash

#Build Flag
PARAM=$1
####################################    Constants    ##################################################

#depends on mainnet or testnet
#NODE="--node https://rpc.junomint.com:443"
#CHAIN_ID=juno-1
#DENOM="ujuno"
#CONTRACT_CREW="juno18cpnn3cnrr9xq7r0cqp7shl7slasf27nrmskw4rrw8c6hyp8u7rqe2nulg"

#NODE="--node https://rpc.juno.giansalex.dev:443"
NODE="--node https://rpc.uni.junomint.com:443"
CHAIN_ID=uni-1
DENOM="ujunox"
CONTRACT_CREW="juno1fjspqgdn4v88rwz9gw8zn4d38fp07cxnrtgw3jtah2j6nymzxgpqqp94xz"

#Another CW20
#BIGTESTADDR="juno1dj87fruymmpk77lrpx2cjqwr5j5ue3kjd3ljee6g4kjj3hxg4yqsrmh5y0"

#not depends
NODECHAIN=" $NODE --chain-id $CHAIN_ID"
TXFLAG=" $NODECHAIN --gas-prices 0.03$DENOM --gas auto --gas-adjustment 1.3"
WALLET="--from workshop"
WASMFILE="artifacts/cw20_escrow.wasm"

FILE_UPLOADHASH="uploadtx.txt"
FILE_WORKSHOP_CONTRACT_ADDR="contractaddr.txt"
FILE_CODE_ID="code.txt"

ADDR_WORKSHOP="juno1htjut8n7jv736dhuqnad5mcydk6tf4ydeaan4s"
ADDR_ACHILLES="juno15fg4zvl8xgj3txslr56ztnyspf3jc7n9j44vhz"
ADDR_ARBITER="juno1m0snhthwl80hweae54fwre97y47urlxjf5ua6j"

###################################################################################################
###################################################################################################
###################################################################################################
###################################################################################################
#Environment Functions
CreateEnv() {
    sudo apt-get update && sudo apt upgrade -y
    sudo apt-get install make build-essential gcc git jq chrony -y
    wget https://golang.org/dl/go1.17.3.linux-amd64.tar.gz
    sudo tar -C /usr/local -xzf go1.17.3.linux-amd64.tar.gz
    rm -rf go1.17.3.linux-amd64.tar.gz

    export GOROOT=/usr/local/go
    export GOPATH=$HOME/go
    export GO111MODULE=on
    export PATH=$PATH:/usr/local/go/bin:$HOME/go/bin
    
    rustup default stable
    rustup target add wasm32-unknown-unknown

    git clone https://github.com/CosmosContracts/juno
    cd juno
    git fetch
    git checkout v2.1.0
    make install

    rm -rf juno

    junod keys import workshop workshop.key

}

#Contract Functions

#Build Optimized Contracts
OptimizeBuild() {

    echo "================================================="
    echo "Optimize Build Start"
    
    docker run --rm -v "$(pwd)":/code \
        --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
        --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
        cosmwasm/rust-optimizer:0.12.4
}

RustBuild() {

    echo "================================================="
    echo "Rust Optimize Build Start"

    cd contracts/cw20-escrow
    RUSTFLAGS='-C link-arg=-s' cargo wasm

    cd ../../
    mkdir artifacts
    cp target/wasm32-unknown-unknown/release/cw20_escrow.wasm $WASMFILE
}

#Writing to FILE_UPLOADHASH
Upload() {
    echo "================================================="
    echo "Upload $WASMFILE"
    
    UPLOADTX=$(junod tx wasm store $WASMFILE $WALLET $TXFLAG --output json -y | jq -r '.txhash')
    echo "Upload txHash:"$UPLOADTX
    
    #save to FILE_UPLOADHASH
    echo $UPLOADTX > $FILE_UPLOADHASH
    echo "wrote last transaction hash to $FILE_UPLOADHASH"
}

#Read code from FILE_UPLOADHASH
GetCode() {
    echo "================================================="
    echo "Get code from transaction hash written on $FILE_UPLOADHASH"
    
    #read from FILE_UPLOADHASH
    TXHASH=$(cat $FILE_UPLOADHASH)
    echo "read last transaction hash from $FILE_UPLOADHASH"
    echo $TXHASH
    
    QUERYTX="junod query tx $TXHASH $NODECHAIN --output json"
	CODE_ID=$(junod query tx $TXHASH $NODECHAIN --output json | jq -r '.logs[0].events[-1].attributes[0].value')
	echo "Contract Code_id:"$CODE_ID

    #save to FILE_CODE_ID
    echo $CODE_ID > $FILE_CODE_ID
}

#Instantiate Contract
Instantiate() {
    echo "================================================="
    echo "Instantiate Contract"
    
    #read from FILE_CODE_ID
    CODE_ID=$(cat $FILE_CODE_ID)
    junod tx wasm instantiate $CODE_ID '{}' --label "WorkShop" $WALLET $TXFLAG -y
}

#Get Instantiated Contract Address
GetContractAddress() {
    echo "================================================="
    echo "Get contract address by code"
    
    #read from FILE_CODE_ID
    CODE_ID=$(cat $FILE_CODE_ID)
    CONTRACT_ADDR=$(junod query wasm list-contract-by-code $CODE_ID $NODECHAIN --output json | jq -r '.contracts[0]')
    
    echo "Contract Address : "$CONTRACT_ADDR

    #save to FILE_WORKSHOP_CONTRACT_ADDR
    echo $CONTRACT_ADDR > $FILE_WORKSHOP_CONTRACT_ADDR
}


###################################################################################################
###################################################################################################
###################################################################################################
###################################################################################################
#Global Utility Functions
ListCode() {
    junod query wasm list-code $NODECHAIN --output json
}

PrintContractState() {
    $CONTRACT_WORKSHOP=$(cat $FILE_WORKSHOP_CONTRACT_ADDR)
    junod query wasm contract $? $NODECHAIN
}

PrintContractStateAll() {
    junod query wasm contract-state all $? $NODECHAIN --output json
}

PrintWalletBalance() {
    junod query wasm contract-state smart $CONTRACT_CREW '{"balance":{"address":"'$ADDR_ACHILLES'"}}' $NODECHAIN
}

###################################################################################################
###################################################################################################
###################################################################################################
###################################################################################################
#Escrow Contract Specific Functions

#Print Escrow List
PrintListQuery() {
    CONTRACT_WORKSHOP=$(cat $FILE_WORKSHOP_CONTRACT_ADDR)
    junod query wasm contract-state smart $CONTRACT_WORKSHOP '{"list":{}}' $NODECHAIN
}

#Print Special Escrow Details
PrintDetailsQuery() {
    CONTRACT_WORKSHOP=$(cat $FILE_WORKSHOP_CONTRACT_ADDR)
    junod query wasm contract-state smart $CONTRACT_WORKSHOP '{"details":{"id":"'$ADDR_ACHILLES'"}}' $NODECHAIN
}

#Print Constants
PrintConstants() {
    CONTRACT_WORKSHOP=$(cat $FILE_WORKSHOP_CONTRACT_ADDR)
    junod query wasm contract-state smart $CONTRACT_WORKSHOP '{"constants":{}}' $NODECHAIN
}

###################################################################################################
###################################################################################################
###################################################################################################
###################################################################################################
#Create Test Escrow
CreateEscrow() {
    CONTRACT_WORKSHOP=$(cat $FILE_WORKSHOP_CONTRACT_ADDR)
    junod tx wasm execute $CONTRACT_WORKSHOP '{"create":{"id":"'$ADDR_ACHILLES'", "arbiter":"'$ADDR_WORKSHOP'", "recipient":"'$ADDR_ACHILLES'"}}' $WALLET $TXFLAG
}

#Transfer to Created Test Escrow
TopUp() {
    CONTRACT_WORKSHOP=$(cat $FILE_WORKSHOP_CONTRACT_ADDR)
    junod tx wasm execute $CONTRACT_WORKSHOP '{"top_up":{"id":"'$ADDR_ACHILLES'"}}' $WALLET $TXFLAG
}

CreateReceive() {
    CONTRACT_WORKSHOP=$(cat $FILE_WORKSHOP_CONTRACT_ADDR)
    junod tx wasm execute $CONTRACT_WORKSHOP '{"receive":{"sender":"'$ADDR_ACHILLES'", "amount":"15", "msg": { "id":"'$ADDR_ACHILLES'", "arbiter":"'$ADDR_ARBITER'", "recipient":"'$ADDR_ACHILLES'" }}}' $WALLET $TXFLAG
}

Approve() {
    CONTRACT_WORKSHOP=$(cat $FILE_WORKSHOP_CONTRACT_ADDR)
    junod tx wasm execute $CONTRACT_WORKSHOP '{"approve":{"id":"'$ADDR_ACHILLES'"}}' $WALLET $TXFLAG
}

Refund() {
    CONTRACT_WORKSHOP=$(cat $FILE_WORKSHOP_CONTRACT_ADDR)
    junod tx wasm execute $CONTRACT_WORKSHOP '{"refund":{"id":"'$ADDR_ACHILLES'"}}' $WALLET $TXFLAG
}


SetConstant() {
    CONTRACT_WORKSHOP=$(cat $FILE_WORKSHOP_CONTRACT_ADDR)
    junod tx wasm execute $CONTRACT_WORKSHOP '{"set_constant":{"manager_addr":"'$ADDR_WORKSHOP'", "min_stake":"10", "rate_client":"20", "rate_manager":"20"}}' $WALLET $TXFLAG
}


#################################### End of Function ###################################################
if [[ $PARAM == "" ]]; then
    RustBuild
    Upload
sleep 5
    GetCode
sleep 5
    Instantiate
sleep 5
    GetContractAddress
sleep 5
    SetConstant
#sleep 5
#    CreateEscrow
# sleep 5
#     TopUp
# sleep 5
#     PrintDetailsQuery
else
    $PARAM
fi

# OptimizeBuild
# Upload
# GetCode
# Instantiate
# GetContractAddress
# CreateEscrow
# TopUp

