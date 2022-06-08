use std::ops::Div;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, SubMsg, WasmMsg, Uint128, Timestamp
};

use cw2::set_contract_version;
use cw20::{Balance, Cw20Coin, Cw20CoinVerified, Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::error::ContractError;
use crate::msg::{
    CreateMsg, TopUpMsg, DetailsResponse, DetailsAllResponse, ExecuteMsg, InstantiateMsg, ListResponse, IsAdminResponse, QueryMsg, ReceiveMsg, ConstantMsg
};
use crate::state::{all_escrow_ids, Escrow, GenericBalance, GenericAccount, ESCROWS, CONSTANT, AccountInfo};

// version info for migration info
const CONTRACT_NAME: &str = "Doodle Workshop";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    // no setup
    
    CONSTANT.save(deps.storage, "manager_addr", &String::from(""))?;
    CONSTANT.save(deps.storage, "min_stake", &String::from("10"))?;
    CONSTANT.save(deps.storage, "rate_client", &String::from("10"))?;
    CONSTANT.save(deps.storage, "rate_manager", &String::from("10"))?;

    CONSTANT.save(deps.storage, "crew_address", &msg.crew_address)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Create(msg) => {
            execute_create(deps, env, msg, Balance::from(info.funds), &info.sender)
        }
        ExecuteMsg::Approve { id} => execute_approve(deps, env, info, id),
        ExecuteMsg::TopUp (msg) => {
            execute_top_up(deps, env, msg, Balance::from(info.funds), &info.sender)
        }
        ExecuteMsg::Refund { id } => execute_refund(deps, env, info, id),
        ExecuteMsg::Remove { id } => execute_remove(deps, env, info, id),
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
        ExecuteMsg::SetConstant(msg) => execute_setconstant(deps, info, msg)
    }
}

pub fn execute_setconstant(
    deps: DepsMut,
    info: MessageInfo,
    msg: ConstantMsg,
) -> Result<Response, ContractError> {
    
    let manager_addr:String = CONSTANT.load(deps.storage, "manager_addr")?;
    let maddr:Addr;
    if manager_addr.ne("") {
        maddr = deps.api.addr_validate(&manager_addr)?;
        if info.sender != maddr {
            return Err(ContractError::Unauthorized {});
        }
    }
    CONSTANT.save(deps.storage, "manager_addr", &msg.manager_addr)?;
    CONSTANT.save(deps.storage, "min_stake", &msg.min_stake)?;
    CONSTANT.save(deps.storage, "rate_client", &msg.rate_client)?;
    CONSTANT.save(deps.storage, "rate_manager", &msg.rate_manager)?;

    let res = Response::new().add_attributes(vec![("action", "setcontant")]);
    Ok(res)
}


pub fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let msg: ReceiveMsg = from_binary(&wrapper.msg)?;
    let balance = Balance::Cw20(Cw20CoinVerified {
        address: info.sender.clone(),
        amount: wrapper.amount,
    });
    let str = CONSTANT.load(deps.storage, "crew_address")?;
    let addr = deps.api.addr_validate(&str)?;
    if info.sender.clone() != addr {
        return Err(ContractError::NotCrew {  });
    }
    let api = deps.api;
    match msg {
        ReceiveMsg::Create(msg) => {
            execute_create(deps, env, msg, balance, &api.addr_validate(&wrapper.sender)?)
        }
        ReceiveMsg::TopUp(msg ) => {
            execute_top_up(deps, env, msg, balance, &api.addr_validate(&wrapper.sender)?)
        }
    }
}

pub fn execute_create(
    deps: DepsMut,
    env: Env,
    msg: CreateMsg,
    balance: Balance,
    sender: &Addr,
) -> Result<Response, ContractError> {
    if balance.is_empty() {
        return Err(ContractError::EmptyBalance {});
    }

    let mut cw20_whitelist = msg.addr_whitelist(deps.api)?;

    let escrow_balance = match balance {
        Balance::Native(balance) => GenericBalance {
            native: balance.0,
            cw20: vec![],
        },
        Balance::Cw20(token) => {

            if !cw20_whitelist.iter().any(|t| t == &token.address) {
                cw20_whitelist.push(token.address.clone())
            }
            GenericBalance {
                native: vec![],
                cw20: vec![token],
            }
        }
    };

    // let account_info = GenericAccount {
    //     account: vec![]
    // };
    let escrow = Escrow {
        //client: deps.api.addr_validate(&msg.client)?,
        client: sender.clone(),
        account_info: String::from(""),
        work_title: msg.work_title,
        work_desc: msg.work_desc,
        work_url: msg.work_url,
        start_time: msg.start_time,
        account_min_stake_amount: msg.account_min_stake_amount,
        stake_amount: msg.stake_amount,
        balance: escrow_balance,
        cw20_whitelist,
        state: 0, // created state
        image_url: msg.image_url
    };

    // try to store it, fail if the id was already in use
    ESCROWS.update(deps.storage, &msg.id, |existing| match existing {
        None => Ok(escrow),
        Some(_) => Err(ContractError::AlreadyInUse {}),
    })?;

    let res = Response::new().add_attributes(vec![("action", "create"), ("id", msg.id.as_str())]);
    Ok(res)
}

pub fn execute_top_up(
    deps: DepsMut,
    env: Env,
    msg: TopUpMsg,
    balance: Balance,
    sender: &Addr
) -> Result<Response, ContractError> {
    if balance.is_empty() {
        return Err(ContractError::EmptyBalance {});
    }
    // this fails is no escrow there
    let mut escrow = ESCROWS.load(deps.storage, &msg.id)?;

    if escrow.is_expired(&env) && escrow.state > 0 {
        return Err(ContractError::StakeFinished {});
    }


    let mut cwval:u128 = 0;
    if let Balance::Cw20(token) = &balance {
        // ensure the token is on the whitelist
        if !escrow.cw20_whitelist.iter().any(|t| t == &token.address) {
            return Err(ContractError::NotInWhitelist {});
        } else {
           cwval = token.amount.u128();
        }
    };
    
    // let account_info:AccountInfo = AccountInfo {
    //     addr: sender.clone(),
    //     amount: cwval,
    //     start_time: msg.start_time,
    //     end_time: msg.end_time
    // };

    let str:String = escrow.account_info + ";" + 
    sender.to_string().as_str() + ":" +
    cwval.to_string().as_str() + ":" +
    msg.start_time.to_string().as_str() + ":" +
    msg.end_time.to_string().as_str();
    escrow.account_info = str;
    // escrow.account_info.add_account(account_info);
    escrow.balance.add_tokens(balance);
    
    if escrow.balance.cw20.get(0).unwrap().amount >= Uint128::from(escrow.stake_amount) /*&& escrow.is_expired(&env)*/ {
        escrow.state = 1; //set to started state
    }
    // and save
    ESCROWS.save(deps.storage, &msg.id, &escrow)?;
    //return Err(ContractError::NotInWhitelist {});
    let res = Response::new().add_attributes(vec![("action", "top_up"), ("id", msg.id.as_str())]);
    Ok(res)
}

pub fn execute_approve (
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: String,
) -> Result<Response, ContractError> {
    // this fails is no escrow there

    let mut escrow = ESCROWS.load(deps.storage, &id)?;
    let manager_addr:String = CONSTANT.load(deps.storage, "manager_addr")?;
    let maddr:Addr = deps.api.addr_validate(&manager_addr)?;

    if escrow.state == 0 || !escrow.is_expired(&env) {
        Err(ContractError::NotStarted {})
    } else if escrow.state == 1 && info.sender != escrow.client {
        Err(ContractError::NotClient {})
    } else if escrow.state == 2 && info.sender != maddr {
        Err(ContractError::NotManager {})
    } else if escrow.state == 3 {
        Err(ContractError::NotLeft {})

    } else {
        let addr:Addr = escrow.balance.cw20.get(0).unwrap().address.clone();
        let mut messages: Vec<SubMsg> = vec![];
        if escrow.state == 1 {
            // First, client must approve
            let rate_manager:Uint128 = CONSTANT.load(deps.storage, "rate_manager")?.parse().unwrap();
            let rate_client:Uint128 = Uint128::from(100u128).checked_sub(rate_manager).unwrap();
            let client_amount:Uint128 = Uint128::from(escrow.stake_amount).checked_mul(rate_client).unwrap().checked_div(Uint128::from(100u128)).unwrap();
            
            let token_client = Cw20CoinVerified {
                address: addr.clone(),
                amount: client_amount
            };
            
            let balance_client = GenericBalance {
                native: vec![],
                cw20: vec![token_client],
            };

            messages = send_tokens(&info.sender, &balance_client)?;
            escrow.balance.sub_tokens(Balance::Cw20(Cw20CoinVerified {
                address: addr.clone(),
                amount: client_amount
            }));
        } else if escrow.state == 2 {
            //send all left tokens to manager
            
            messages = send_tokens(&info.sender, &escrow.balance)?;
            escrow.balance.sub_tokens(Balance::Cw20(Cw20CoinVerified {
                address: addr.clone(),
                amount: escrow.balance.cw20.get(0).unwrap().amount
            }));
        }
 
        escrow.state = escrow.state + 1;

        ESCROWS.save(deps.storage, &id, &escrow)?;
        
        Ok(Response::new()
            .add_attribute("action", "approve")
            .add_attribute("id", id)
            .add_attribute("to", info.sender)
            .add_submessages(messages))
    }
}

pub fn accountStaked(deps:Deps, account_info:&String, addr:Addr) -> (String, String) {
    let accounts: Vec<&str> = account_info.split(';').collect();

    for account in accounts {
        let infos:Vec<&str> = account.split(':').collect();
        
        if infos.len() != 4 || deps.api.addr_validate(&infos[0]).unwrap() != addr {
            continue;
        }
        return (String::from(account), String::from(infos[1]));
    }
    (String::from(""), String::from(""))
}

pub fn execute_refund(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: String,
) -> Result<Response, ContractError> {
    // this fails is no escrow there
    let mut escrow = ESCROWS.load(deps.storage, &id)?;

    let mut messages: Vec<SubMsg> = vec![];
    if escrow.state > 0 && escrow.is_expired(&env) {
        Err(ContractError::AlreadyStarted {})
    } else if !escrow.is_expired(&env) {
        Err(ContractError::WorkNotExpired {})
    } else if escrow.account_info.len() == 0 {
        Err(ContractError::NobodyStaked {})
    } else {
        // we delete the escrow
        // ESCROWS.remove(deps.storage, &id);
        let mut account_info:String = escrow.account_info.clone();

        let accounts: Vec<&str> = account_info.split(';').collect();
        let mut exist:bool = false;

        let mut newaccoount_info:String = String::from("");
        for account in accounts {
            let infos:Vec<&str> = account.split(':').collect();
            if infos.len() == 4 && deps.api.addr_validate(&infos[0])? != info.sender{
                newaccoount_info = newaccoount_info + ";" + account;
            }
            if infos.len() != 4 || deps.api.addr_validate(&infos[0])? != info.sender {
                continue;
            }
            let sender:Addr = deps.api.addr_validate(&infos[0])?;
            let cwval = infos[1];
            let start_time = infos[2];
            let end_time = infos[3];
            exist = true;

            if env.block.time < Timestamp::from_seconds(end_time.parse().unwrap()) &&
            env.block.time > Timestamp::from_seconds(start_time.parse().unwrap())
            {
                return Err(ContractError::AccountNotExpired {});
            } else {
                let addr:Addr = escrow.balance.cw20.get(0).unwrap().address.clone();
                let token_account = Cw20CoinVerified {
                    address: addr.clone(),
                    amount: cwval.parse().unwrap()
                };
        
                let balance_account = GenericBalance {
                    native: vec![],
                    cw20: vec![token_account],
                };
                
                messages = send_tokens(&sender, &balance_account)?;

                //remove tokens from escrow.balance
                escrow.balance.sub_tokens(Balance::Cw20(Cw20CoinVerified {
                    address: addr.clone(),
                    amount: cwval.parse().unwrap()
                }));
            }
            
        }

        if !exist {
            return Err(ContractError::DidntStaked {});
        } 
        escrow.account_info = newaccoount_info;
        if escrow.balance.cw20.get(0).unwrap().amount >= Uint128::from(escrow.stake_amount) /*&& escrow.is_expired(&env)*/ {
            escrow.state = 1; //set to started state
        } else {
            escrow.state = 0;
        }

        ESCROWS.save(deps.storage, &id, &escrow)?;
        Ok(Response::new()
        .add_attribute("action", "refund")
        .add_attribute("id", id)
        .add_attribute("to", info.sender)
        .add_submessages(messages))
    }
}


pub fn execute_remove(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: String,
) -> Result<Response, ContractError> {
    // this fails is no escrow there

    let mut escrow = ESCROWS.load(deps.storage, &id)?;
    let manager_addr:String = CONSTANT.load(deps.storage, "manager_addr")?;
    let maddr:Addr = deps.api.addr_validate(&manager_addr)?;

    if escrow.state != 3 {
        Err(ContractError::NotFinished {})
    } else if info.sender != maddr {
        Err(ContractError::NotManager {})
    } else {
        // we delete the escrow
        ESCROWS.remove(deps.storage, &id);

        Ok(Response::new()
        .add_attribute("action", "remove")
        .add_attribute("id", id))
    }
}

fn send_tokens(to: &Addr, balance: &GenericBalance) -> StdResult<Vec<SubMsg>> {
    let native_balance = &balance.native;
    let mut msgs: Vec<SubMsg> = if native_balance.is_empty() {
        vec![]
    } else {
        vec![SubMsg::new(BankMsg::Send {
            to_address: to.into(),
            amount: native_balance.to_vec(),
        })]
    };

    let cw20_balance = &balance.cw20;
    let cw20_msgs: StdResult<Vec<_>> = cw20_balance
        .iter()
        .map(|c| {
            let msg = Cw20ExecuteMsg::Transfer {
                recipient: to.into(),
                amount: c.amount,
            };
            let exec = SubMsg::new(WasmMsg::Execute {
                contract_addr: c.address.to_string(),
                msg: to_binary(&msg)?,
                funds: vec![],
            });
            Ok(exec)
        })
        .collect();
    msgs.append(&mut cw20_msgs?);
    Ok(msgs)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::List {} => to_binary(&query_list(deps)?),
        QueryMsg::DetailsAll {addr} => to_binary(&query_detailsall(deps, env, addr)?),
        QueryMsg::Constants {} => to_binary(&query_constants(deps)?),
        QueryMsg::IsAdmin {addr} => to_binary(&query_isadmin(deps, addr)?),
    }
}

fn query_list(deps: Deps) -> StdResult<ListResponse> {
    Ok(ListResponse {
        escrows: all_escrow_ids(deps.storage)?,
    })
}

fn query_detailsall(deps: Deps, env: Env, addr:String) -> StdResult<DetailsAllResponse> {
    let ids:Vec<String> = all_escrow_ids(deps.storage)?;

    let isadmin:bool = CONSTANT.load(deps.storage, "manager_addr")? == addr || CONSTANT.load(deps.storage, "manager_addr")? == "";

    let mut ret:Vec<DetailsResponse> = vec![];

    for idstr in ids {
        let escrow = ESCROWS.load(deps.storage, idstr.as_str())?;
        let expired:bool = escrow.is_expired(&env);
        //let expired:bool = false;
        let cw20_balance: StdResult<Vec<_>> = escrow
            .balance
            .cw20
            .into_iter()
            .map(|token| {
                Ok(Cw20Coin {
                    address: token.address.into(),
                    amount: token.amount,
                })
            })
            .collect();
        
        let mut accountinfo:String = String::from(escrow.account_info);
        let (my_staked_all, my_staked) = accountStaked(deps, &accountinfo, deps.api.addr_validate(&addr)?);
        
        if !isadmin {
            accountinfo = my_staked_all;
        }

        let mut workurl = String::from("");
        if isadmin || escrow.state > 0 && my_staked != String::from("") && expired || escrow.client == addr {
            workurl = escrow.work_url;
        }
        let mut cw20balance = vec![];
        if isadmin {
            cw20balance = cw20_balance?;
        }
        
        let details = DetailsResponse {
            id: idstr,
            client: escrow.client.into(),
            work_title: escrow.work_title,
            work_desc: escrow.work_desc,
            work_url: workurl,
            start_time: escrow.start_time,
            account_min_stake_amount: escrow.account_min_stake_amount,
            stake_amount: escrow.stake_amount,
            cw20_balance: cw20balance,
            account_info: accountinfo,
            state: escrow.state,
            my_staked: my_staked,
            expired: expired,
            timestamp: env.block.time.seconds().to_string(),
            image_url: escrow.image_url
        };
        ret.push(details);
    }
    
    Ok(DetailsAllResponse {
        escrows: ret
    })
}

fn query_constants(deps: Deps) -> StdResult<ConstantMsg> {

    Ok(ConstantMsg {
        manager_addr: CONSTANT.load(deps.storage, "manager_addr")?,
        min_stake: CONSTANT.load(deps.storage, "min_stake")?,
        rate_client: CONSTANT.load(deps.storage, "rate_client")?,
        rate_manager: CONSTANT.load(deps.storage, "rate_manager")?,
    })
}

fn query_isadmin(deps: Deps, addr: String) -> StdResult<IsAdminResponse> {

    let manager_addr:String  = CONSTANT.load(deps.storage, "manager_addr")?;

    Ok(IsAdminResponse {
        isadmin: manager_addr == "" || manager_addr == addr,
    })
}
