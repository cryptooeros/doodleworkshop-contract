#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, SubMsg, WasmMsg, Order
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
            execute_create(deps, msg, Balance::from(info.funds), &info.sender)
        }
        ExecuteMsg::Approve { id } => execute_approve(deps, env, info, id),
        ExecuteMsg::TopUp (msg) => {
            execute_top_up(deps, msg, Balance::from(info.funds), &info.sender)
        }
        ExecuteMsg::Refund { id } => execute_refund(deps, env, info, id),
        ExecuteMsg::Receive(msg) => execute_receive(deps, info, msg),
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
            execute_create(deps, msg, balance, &api.addr_validate(&wrapper.sender)?)
        }
        ReceiveMsg::TopUp(msg ) => {
            execute_top_up(deps, msg, balance, &api.addr_validate(&wrapper.sender)?)
        }
    }
}

pub fn execute_create(
    deps: DepsMut,
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

    let account_info = GenericAccount {
        account: vec![]
    };
    let escrow = Escrow {
        //client: deps.api.addr_validate(&msg.client)?,
        client: sender.clone(),
        account_info: account_info,
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
    msg: TopUpMsg,
    balance: Balance,
    sender: &Addr
) -> Result<Response, ContractError> {
    if balance.is_empty() {
        return Err(ContractError::EmptyBalance {});
    }
    // this fails is no escrow there
    let mut escrow = ESCROWS.load(deps.storage, &msg.id)?;

    let mut cwval:u128 = 0;
    if let Balance::Cw20(token) = &balance {
        // ensure the token is on the whitelist
        if !escrow.cw20_whitelist.iter().any(|t| t == &token.address) {
            return Err(ContractError::NotInWhitelist {});
        } else {
           cwval = token.amount.u128();
        }
    };

    
    
    let account_info:AccountInfo = AccountInfo {
        addr: sender.clone(),
        amount: cwval,
        start_time: msg.start_time,
        end_time: msg.end_time
    };

    escrow.account_info.add_account(account_info);
    escrow.balance.add_tokens(balance);

    // and save
    ESCROWS.save(deps.storage, &msg.id, &escrow)?;

    let res = Response::new().add_attributes(vec![("action", "top_up"), ("id", msg.id.as_str())]);
    Ok(res)
}

pub fn execute_approve(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: String,
) -> Result<Response, ContractError> {
    // this fails is no escrow there
    let escrow = ESCROWS.load(deps.storage, &id)?;

    let manager_addr:String = CONSTANT.load(deps.storage, "manager_addr")?;
    let maddr:Addr = deps.api.addr_validate(&manager_addr)?;
    
    if info.sender != escrow.client || info.sender != maddr {
        Err(ContractError::Unauthorized {})
    // } else if escrow.is_expired(&env) {
    //     Err(ContractError::Expired {})
    } else {
        // we delete the escrow
        // ESCROWS.remove(deps.storage, &id);

        // send all tokens out
        let manager_addr:String = CONSTANT.load(deps.storage, "manager_addr")?;
        let maddr:Addr = deps.api.addr_validate(&manager_addr)?;
        let messages1: Vec<SubMsg> = send_tokens(&escrow.client, &escrow.balance)?;
        let messages2: Vec<SubMsg> = send_tokens(&maddr, &escrow.balance)?;

        Ok(Response::new()
            .add_attribute("action", "approve")
            .add_attribute("id", id)
            .add_attribute("to", escrow.client)
            .add_submessages(messages1))
    }
}

pub fn execute_refund(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: String,
) -> Result<Response, ContractError> {
    // this fails is no escrow there
    let escrow = ESCROWS.load(deps.storage, &id)?;

    let manager_addr:String = CONSTANT.load(deps.storage, "manager_addr")?;
    let maddr:Addr = deps.api.addr_validate(&manager_addr)?;
    
    // if !escrow.is_expired(&env) || info.sender != escrow.source {
    //     Err(ContractError::Unauthorized {})
    // } else {
        // we delete the escrow
        // ESCROWS.remove(deps.storage, &id);

        // send all tokens out
        let messages = send_tokens(&info.sender, &escrow.balance)?;

        Ok(Response::new()
            .add_attribute("action", "refund")
            .add_attribute("id", id)
            .add_attribute("to", escrow.client)
            .add_submessages(messages))
    // }
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
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::List {} => to_binary(&query_list(deps)?),
        QueryMsg::DetailsAll {} => to_binary(&query_detailsall(deps)?),
        QueryMsg::Details { id } => to_binary(&query_details(deps, id)?),
        QueryMsg::Constants {} => to_binary(&query_constants(deps)?),
        QueryMsg::IsAdmin {addr} => to_binary(&query_isadmin(deps, addr)?),
    }
}

fn query_details(deps: Deps, id: String) -> StdResult<DetailsResponse> {
    let escrow = ESCROWS.load(deps.storage, &id)?;

    let cw20_whitelist = escrow.human_whitelist();

    // transform tokens
    let native_balance = escrow.balance.native;

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

    let details = DetailsResponse {
        id,
        client: escrow.client.into(),
        work_title: escrow.work_title,
        work_desc: escrow.work_desc,
        work_url: escrow.work_url,
        start_time: escrow.start_time,
        account_min_stake_amount: escrow.account_min_stake_amount,
        stake_amount: escrow.stake_amount,
        cw20_balance: cw20_balance?,
        account_info: escrow.account_info.account,
        state: escrow.state
    };
    Ok(details)
}

fn query_list(deps: Deps) -> StdResult<ListResponse> {
    Ok(ListResponse {
        escrows: all_escrow_ids(deps.storage)?,
    })
}

fn query_detailsall(deps: Deps) -> StdResult<DetailsAllResponse> {
    let ids:Vec<String> = all_escrow_ids(deps.storage)?;
    let mut ret:Vec<DetailsResponse> = vec![];

    for idstr in ids {
        let escrow = ESCROWS.load(deps.storage, idstr.as_str())?;

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
        
        let details = DetailsResponse {
            id: idstr,
            client: escrow.client.into(),
            work_title: escrow.work_title,
            work_desc: escrow.work_desc,
            work_url: escrow.work_url,
            start_time: escrow.start_time,
            account_min_stake_amount: escrow.account_min_stake_amount,
            stake_amount: escrow.stake_amount,
            cw20_balance: cw20_balance?,
            account_info: escrow.account_info.account,
            state: escrow.state
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
        let details = query_details(deps.as_ref(), "foobar".to_string()).unwrap();
        assert_eq!(
            details,
            DetailsResponse {
                id: "foobar".to_string(),
                client: String::from("arbitrate"),
                start_time: Some(123456),
                work_title: String::from("title"),
                work_url: String::from("url"),
                work_desc: String::from("desc"),
                account_min_stake_amount: 10,
                stake_amount: 100,
                cw20_balance: vec![Cw20Coin {
                    address: String::from("my-cw20-token"),
                    amount: Uint128::new(100),
                }],
                account_info: vec![],
                state: 0
            }
        );

        // approve it
        let id = create.id.clone();
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
        let id = create.id.clone();
        let info = mock_info(&create.client, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Approve { id }).unwrap_err();
        assert!(matches!(err, ContractError::Std(StdError::NotFound { .. })));
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
        
        // let top_up = ExecuteMsg::TopUp {
        //     msg:to_binary(&send_msg).unwrap()
        // };
        // let res = execute(deps.as_mut(), mock_env(), info, top_up).unwrap();
        // assert_eq!(0, res.messages.len());
        // assert_eq!(("action", "top_up"), res.attributes[0]);

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
