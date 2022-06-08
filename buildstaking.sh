#!/bin/bash

#Build Flag
PARAM=$1
####################################    Constants    ##################################################

#depends on mainnet or testnet
NODE="--node https://rpc-juno.itastakers.com:443"
CHAIN_ID=juno-1
DENOM="ujuno"
CONTRACT_CREW="juno1ugc7q54m0vtyc05kfmn69g8zx55z8s9mz5u05v3cden7yjqdxd4sk52wac"

##########################################################################################

# NODE="--node https://rpc.juno.giansalex.dev:443"
# CHAIN_ID=uni-3
# DENOM="ujunox"
##########################################################################################
# CONTRACT_CREW="juno1fjspqgdn4v88rwz9gw8zn4d38fp07cxnrtgw3jtah2j6nymzxgpqqp94xz"

#not depends
NODECHAIN=" $NODE --chain-id $CHAIN_ID"
TXFLAG=" $NODECHAIN --gas-prices 0.01$DENOM --gas auto --gas-adjustment 1.3"
WALLET="--from workshop"

RELEASE="release/"
WASMRAWFILE="doodle.wasm"
WASMFILE=$RELEASE$WASMRAWFILE
FILE_UPLOADHASH="uploadtx.txt"
FILE_WORKSHOP_CONTRACT_ADDR="contractaddr.txt"
FILE_CODE_ID="code.txt"

ADDR_WORKSHOP="juno1htjut8n7jv736dhuqnad5mcydk6tf4ydeaan4s"
ADDR_ARBITER="juno1htjut8n7jv736dhuqnad5mcydk6tf4ydeaan4s"
ADDR_ADMIN=ADDR_WORKSHOP

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
    git checkout v6.0.0
    make install
    cd ../
    rm -rf juno
}

Upload() {
    echo "================================================="
    echo "Rust Optimize Build Start"
    mkdir release
    # RUSTFLAGS='-C link-arg=-s' cargo wasm
    # cp target/wasm32-unknown-unknown/$WASMFILE $WASMFILE

    docker run --rm -v "$(pwd)":/code \
    --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
    --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
    cosmwasm/rust-optimizer:0.12.6
    cp artifacts/$WASMRAWFILE $WASMFILE
    
    

    echo "================================================="
    echo "Upload $WASMFILE"
    
    UPLOADTX=$(junod tx wasm store $WASMFILE $WALLET $TXFLAG --output json -y | jq -r '.txhash')
    echo "Upload txHash:"$UPLOADTX
    
    echo "================================================="
    echo "GetCode"
	CODE_ID=""
    while [[ $CODE_ID == "" ]]
    do 
        sleep 3
        CODE_ID=$(junod query tx $UPLOADTX $NODECHAIN --output json | jq -r '.logs[0].events[-1].attributes[0].value')
    done
    echo "Contract Code_id:"$CODE_ID

    #save to FILE_CODE_ID
    echo $CODE_ID > $FILE_CODE_ID
}

Instantiate() { 
    echo "================================================="
    echo "Instantiate Contract"
    #read from FILE_CODE_ID
    CODE_ID=$(cat $FILE_CODE_ID)
    echo $CODE_ID
    #read from FILE_CODE_ID
    
    TXHASH=$(junod tx wasm instantiate $CODE_ID '{"crew_address":"'$CONTRACT_CREW'"}' --label "DoodleWorkshop" --admin $ADDR_ADMIN $WALLET $TXFLAG -y --output json | jq -r '.txhash')
    echo $TXHASH
    CONTRACT_ADDR=""
    while [[ $CONTRACT_ADDR == "" ]]
    do
        sleep 3
        CONTRACT_ADDR=$(junod query tx $TXHASH $NODECHAIN --output json | jq -r '.logs[0].events[0].attributes[0].value')
    done
    echo $CONTRACT_ADDR
    echo $CONTRACT_ADDR > $FILE_CONTRACT_ADDR
}

###################################################################################################
###################################################################################################
###################################################################################################
###################################################################################################
#Escrow Contract Specific Functions

#Print Escrow List
PrintListQuery() {
    CONTRACT_WORKSHOP=$(cat $FILE_WORKSHOP_CONTRACT_ADDR)
    junod query wasm contract-state smart $CONTRACT_WORKSHOP '{"details_all":{"addr":"'$ADDR_WORKSHOP'"}}' $NODECHAIN
}

PrintIsAdmin() {
    CONTRACT_WORKSHOP=$(cat $FILE_WORKSHOP_CONTRACT_ADDR)
    junod query wasm contract-state smart $CONTRACT_WORKSHOP '{"is_admin":{"addr":"'$ADDR_WORKSHOP'"}}' $NODECHAIN
}

#Print Special Escrow Details
PrintDetailsQuery() {
    CONTRACT_WORKSHOP=$(cat $FILE_WORKSHOP_CONTRACT_ADDR)
    junod query wasm contract-state smart $CONTRACT_WORKSHOP '{"details":{"id":"'$ADDR_WORKSHOP'"}}' $NODECHAIN
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
    junod tx wasm execute $CONTRACT_WORKSHOP '{"set_constant":{"manager_addr":"'$ADDR_WORKSHOP'", "min_stake":"0.01", "rate_client":"10", "rate_manager":"10"}}' $WALLET $TXFLAG
}


#################################### End of Function ###################################################
if [[ $PARAM == "" ]]; then
    RustBuild
    Upload
sleep 5
    GetCode
sleep 5
    Instantiate
sleep 8
    GetContractAddress
sleep 5
    SetConstant
sleep 5
    PrintListQuery
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

