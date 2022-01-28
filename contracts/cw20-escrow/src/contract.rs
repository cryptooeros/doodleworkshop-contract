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
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    // no setup
    
    CONSTANT.save(deps.storage, "manager_addr", &String::from(""))?;
    CONSTANT.save(deps.storage, "min_stake", &String::from("10"))?;
    CONSTANT.save(deps.storage, "rate_client", &String::from("10"))?;
    CONSTANT.save(deps.storage, "rate_manager", &String::from("10"))?;
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
        address: info.sender,
        amount: wrapper.amount,
    });
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
        state: 0 // created state
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

pub fn accountStaked(deps:Deps, account_info:&String, addr:Addr) -> u128 {
    let accounts: Vec<&str> = account_info.split(';').collect();
    let mut exist:bool = false;

    for account in accounts {
        let infos:Vec<&str> = account.split(':').collect();
        
        if infos.len() != 4 || deps.api.addr_validate(&infos[0]).unwrap() != addr {
            continue;
        }
        return infos[1].parse().unwrap();
    }
    0u128
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

            if env.block.time > Timestamp::from_seconds(end_time.parse().unwrap()) ||
            env.block.time < Timestamp::from_seconds(start_time.parse().unwrap())
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
        let my_staked:u128 = accountStaked(deps, &accountinfo, deps.api.addr_validate(&addr)?);
        
        if !isadmin {
            accountinfo = String::from("");
        }

        let mut workurl = String::from("");
        if isadmin || escrow.state > 0 && my_staked > 0u128 && expired || escrow.client == addr {
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
            expired: expired
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

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, coins, CosmosMsg, StdError, Uint128};

    use crate::msg::ExecuteMsg::TopUp;

    use super::*;
    #[test]
    fn happy_path_cw20() {
        let mut deps = mock_dependencies();

        // instantiate an empty contract
        let instantiate_msg = InstantiateMsg {};
        let info = mock_info(&String::from("anyone"), &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // create an escrow
        let create = CreateMsg {
            id: "foobar".to_string(),
            client: String::from("arbitrate"),
            start_time: Some(123456),
            cw20_whitelist: Some(vec![String::from("other-token")]),
            work_title: String::from("title"),
            work_url: String::from("url"),
            work_desc: String::from("desc"),
            account_min_stake_amount: 10,
            stake_amount: 100,
        };
        let receive = Cw20ReceiveMsg {
            sender: String::from("source"),
            amount: Uint128::new(100),
            msg: to_binary(&ExecuteMsg::Create(create.clone())).unwrap(),
        };
        let token_contract = String::from("my-cw20-token");
        let info = mock_info(&token_contract, &[]);
        let msg = ExecuteMsg::Receive(receive.clone());
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(("action", "create"), res.attributes[0]);

        // ensure the whitelist is what we expect
        // let details = query_list(deps.as_ref()).unwrap();
        // assert_eq!(
        //     details,
        //     DetailsResponse {
        //         id: "foobar".to_string(),
        //         client: String::from("arbitrate"),
        //         start_time: Some(123456),
        //         work_title: String::from("title"),
        //         work_url: String::from("url"),
        //         work_desc: String::from("desc"),
        //         account_min_stake_amount: 10,
        //         stake_amount: 100,
        //         cw20_balance: vec![Cw20Coin {
        //             address: String::from("my-cw20-token"),
        //             amount: Uint128::new(100),
        //         }],
        //         account_info: String::from(""),
        //         state: 0,
        //         my_staked: 0u128,
        //         expired: false
        //     }
        // );

        // approve it
        let id = create.id.clone();
        let amount:u128 = 10;
        let info = mock_info(&create.client, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Approve { id }).unwrap();
        assert_eq!(1, res.messages.len());
        assert_eq!(("action", "approve"), res.attributes[0]);
        let send_msg = Cw20ExecuteMsg::Transfer {
            recipient: create.client,
            amount: receive.amount,
        };
        assert_eq!(
            res.messages[0],
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_contract,
                msg: to_binary(&send_msg).unwrap(),
                funds: vec![]
            }))
        );

        // second attempt fails (not found)
        // let id = create.id.clone();
        // let info = mock_info(&create.client, &[]);
        // let err = execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Approve { id }).unwrap_err();
        // assert!(matches!(err, ContractError::Std(StdError::NotFound { .. })));
    }

    #[test]
    fn add_tokens_proper() {
        let mut tokens = GenericBalance::default();
        tokens.add_tokens(Balance::from(vec![coin(123, "atom"), coin(789, "eth")]));
        tokens.add_tokens(Balance::from(vec![coin(456, "atom"), coin(12, "btc")]));
        assert_eq!(
            tokens.native,
            vec![coin(579, "atom"), coin(789, "eth"), coin(12, "btc")]
        );
    }

    #[test]
    fn add_cw_tokens_proper() {
        let mut tokens = GenericBalance::default();
        let bar_token = Addr::unchecked("bar_token");
        let foo_token = Addr::unchecked("foo_token");
        tokens.add_tokens(Balance::Cw20(Cw20CoinVerified {
            address: foo_token.clone(),
            amount: Uint128::new(12345),
        }));
        tokens.add_tokens(Balance::Cw20(Cw20CoinVerified {
            address: bar_token.clone(),
            amount: Uint128::new(777),
        }));
        tokens.add_tokens(Balance::Cw20(Cw20CoinVerified {
            address: foo_token.clone(),
            amount: Uint128::new(23400),
        }));
        assert_eq!(
            tokens.cw20,
            vec![
                Cw20CoinVerified {
                    address: foo_token,
                    amount: Uint128::new(35745),
                },
                Cw20CoinVerified {
                    address: bar_token,
                    amount: Uint128::new(777),
                }
            ]
        );
    }

    #[test]
    fn top_up_mixed_tokens() {
        let mut deps = mock_dependencies();

        // instantiate an empty contract
        let instantiate_msg = InstantiateMsg {};
        let info = mock_info(&String::from("anyone"), &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // only accept these tokens
        let whitelist = vec![String::from("bar_token"), String::from("foo_token")];

        // create an escrow with 2 native tokens
        let create = CreateMsg {
            id: "foobar".to_string(),
            client: String::from("arbitrate"),
            start_time: None,
            cw20_whitelist: Some(whitelist),
            work_title: String::from("title"),
            work_url: String::from("url"),
            work_desc: String::from("desc"),
            account_min_stake_amount: 10,
            stake_amount: 100,
        };
        let sender = String::from("source");
        let balance = vec![coin(100, "fee"), coin(200, "stake")];
        let info = mock_info(&sender, &balance);
        let msg = ExecuteMsg::Create(create.clone());
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(("action", "create"), res.attributes[0]);

        // top it up with 2 more native tokens
        let extra_native = vec![coin(250, "random"), coin(300, "stake")];
        let info = mock_info(&sender, &extra_native);
        
        let top_up = TopUpMsg {
            id: "foobar".to_string(),
            start_time: 123456,
            end_time: 123456,
        };
        let sender = String::from("source");
        let balance = vec![coin(100, "fee"), coin(200, "stake")];
        let info = mock_info(&sender, &balance);
        let msg = ExecuteMsg::TopUp(top_up.clone());
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        
        assert_eq!(0, res.messages.len());
        assert_eq!(("action", "top_up"), res.attributes[0]);

        // // top up with one foreign token
        // let bar_token = String::from("bar_token");
        // let base = TopUp {
        //     msg:to_binary(&send_msg).unwrap()
        // };
        // let top_up = ExecuteMsg::Receive(Cw20ReceiveMsg {
        //     sender: String::from("random"),
        //     amount: Uint128::new(7890),
        //     msg: to_binary(&base).unwrap(),
        // });
        // let info = mock_info(&bar_token, &[]);
        // let res = execute(deps.as_mut(), mock_env(), info, top_up).unwrap();
        // assert_eq!(0, res.messages.len());
        // assert_eq!(("action", "top_up"), res.attributes[0]);

        // // top with a foreign token not on the whitelist
        // // top up with one foreign token
        // let baz_token = String::from("baz_token");
        // let base = TopUp {
        //     id: create.id.clone(),
        //     start_time: 123456,
        //     end_time: 654321
        // };
        // let top_up = ExecuteMsg::Receive(Cw20ReceiveMsg {
        //     sender: String::from("random"),
        //     amount: Uint128::new(7890),
        //     msg: to_binary(&base).unwrap(),
        // });
        // let info = mock_info(&baz_token, &[]);
        // let err = execute(deps.as_mut(), mock_env(), info, top_up).unwrap_err();
        // assert_eq!(err, ContractError::NotInWhitelist {});

        // // top up with second foreign token
        // let foo_token = String::from("foo_token");
        // let base = TopUp {
        //     id: create.id.clone(),
        //     start_time: 123456,
        //     end_time: 654321
        // };
        // let top_up = ExecuteMsg::Receive(Cw20ReceiveMsg {
        //     sender: String::from("random"),
        //     amount: Uint128::new(888),
        //     msg: to_binary(&base).unwrap(),
        // });
        // let info = mock_info(&foo_token, &[]);
        // let res = execute(deps.as_mut(), mock_env(), info, top_up).unwrap();
        // assert_eq!(0, res.messages.len());
        // assert_eq!(("action", "top_up"), res.attributes[0]);

        // // approve it
        // let id = create.id.clone();
        // let info = mock_info(&create.client, &[]);
        // let res = execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Approve { id }).unwrap();
        // assert_eq!(("action", "approve"), res.attributes[0]);
        // assert_eq!(3, res.messages.len());

        // // first message releases all native coins
        // assert_eq!(
        //     res.messages[0],
        //     SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
        //         to_address: create.client.clone(),
        //         amount: vec![coin(100, "fee"), coin(500, "stake"), coin(250, "random")],
        //     }))
        // );

        // // second one release bar cw20 token
        // let send_msg = Cw20ExecuteMsg::Transfer {
        //     recipient: create.client.clone(),
        //     amount: Uint128::new(7890),
        // };
        // assert_eq!(
        //     res.messages[1],
        //     SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        //         contract_addr: bar_token,
        //         msg: to_binary(&send_msg).unwrap(),
        //         funds: vec![]
        //     }))
        // );

        // // third one release foo cw20 token
        // let send_msg = Cw20ExecuteMsg::Transfer {
        //     recipient: create.client,
        //     amount: Uint128::new(888),
        // };
        // assert_eq!(
        //     res.messages[2],
        //     SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        //         contract_addr: foo_token,
        //         msg: to_binary(&send_msg).unwrap(),
        //         funds: vec![]
        //     }))
        // );
    }
}
