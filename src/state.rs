use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Coin, Env, Order, StdResult, Storage, Timestamp};
use cw_storage_plus::Map;

use cw20::{Balance, Cw20CoinVerified};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct GenericBalance {
    pub native: Vec<Coin>,
    pub cw20: Vec<Cw20CoinVerified>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct GenericAccount {
    pub account: Vec<AccountInfo>
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AccountInfo {
    pub addr: Addr,
    pub amount: u128,
    pub start_time: u64,
    pub end_time: u64
}
impl GenericAccount {
    pub fn add_account(&mut self, add: AccountInfo) {
        let index = self.account.iter().enumerate().find_map(|(i, exist)| {
            if exist.addr == add.addr {
                None
            } else {
                Some(i)
            }
        });
        match index {
            Some(idx) => self.account[idx].amount += add.amount,
            None => self.account.push(add),
        };
    }
}


impl GenericBalance {
    pub fn add_tokens(&mut self, add: Balance) {
        match add {
            Balance::Native(balance) => {
                for token in balance.0 {
                    let index = self.native.iter().enumerate().find_map(|(i, exist)| {
                        if exist.denom == token.denom {
                            Some(i)
                        } else {
                            None
                        }
                    });
                    match index {
                        Some(idx) => self.native[idx].amount += token.amount,
                        None => self.native.push(token),
                    }
                }
            }
            Balance::Cw20(token) => {
                let index = self.cw20.iter().enumerate().find_map(|(i, exist)| {
                    if exist.address == token.address {
                        Some(i)
                    } else {
                        None
                    }
                });
                match index {
                    Some(idx) => self.cw20[idx].amount += token.amount,
                    None => self.cw20.push(token),
                }
            }
        };
    }

    pub fn sub_tokens(&mut self, sub: Balance) {
        match sub {
            Balance::Native(balance) => {
                for token in balance.0 {
                    let index = self.native.iter().enumerate().find_map(|(i, exist)| {
                        if exist.denom == token.denom {
                            Some(i)
                        } else {
                            None
                        }
                    });
                    match index {
                        Some(idx) => self.native[idx].amount -= token.amount,
                        None => self.native.push(token),
                    }
                }
            }
            Balance::Cw20(token) => {
                let index = self.cw20.iter().enumerate().find_map(|(i, exist)| {
                    if exist.address == token.address {
                        Some(i)
                    } else {
                        None
                    }
                });
                match index {
                    Some(idx) => self.cw20[idx].amount -= token.amount,
                    None => self.cw20.push(token),
                }
            }
        };
    }

}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Escrow {
    /// client can decide to approve or refund the escrow
    pub client: Addr,
    /// if approved, funds go to the recipient
    // pub account_info: GenericAccount,
    pub account_info: String,
    pub work_title: String,
    pub work_desc: String,
    pub work_url: String,
    pub start_time: Option<u64>,
    pub account_min_stake_amount: u64,
    pub stake_amount: u64,
    /// Balance in Native and Cw20 tokens
    pub balance: GenericBalance,
    /// All possible contracts that we accept tokens from
    pub cw20_whitelist: Vec<Addr>,
    pub state: u8,
    pub image_url: String
}

impl Escrow {
    pub fn is_expired(&self, env: &Env) -> bool {

        if let Some(start_time) = self.start_time {
            if env.block.time > Timestamp::from_seconds(start_time) {
                return true;
            }
        }
        false
    }

    pub fn human_whitelist(&self) -> Vec<String> {
        self.cw20_whitelist.iter().map(|a| a.to_string()).collect()
    }
}

pub const ESCROWS: Map<&str, Escrow> = Map::new("escrow");
pub const CONSTANT: Map<&str, String> = Map::new("constant");
/// This returns the list of ids for all registered escrows
pub fn all_escrow_ids(storage: &dyn Storage) -> StdResult<Vec<String>> {
    ESCROWS
        .keys(storage, None, None, Order::Ascending)
        .collect()
}
