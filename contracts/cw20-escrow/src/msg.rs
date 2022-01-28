use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Api, Coin, StdResult};
use crate::state::{AccountInfo};
use cw20::{Cw20Coin, Cw20ReceiveMsg};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Create(CreateMsg),
    /// Adds all sent native tokens to the contract
    TopUp(TopUpMsg),
    /// Approve sends all tokens to the recipient.
    /// Only the client can do this
    Approve {
        /// id is a human-readable name for the escrow from create
        id: String,
    },
    /// Refund returns all remaining tokens to the original sender,
    /// The client can do this any time, or anyone can do this after a timeout
    Refund {
        /// id is a human-readable name for the escrow from create
        id: String,
    },
    Remove {
        id: String,
    },
    /// This accepts a properly-encoded ReceiveMsg from a cw20 contract
    Receive(Cw20ReceiveMsg),
    /// Set Constant
    SetConstant(ConstantMsg),
    
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReceiveMsg {
    Create(CreateMsg),
    /// Adds all sent native tokens to the contract
    TopUp(TopUpMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CreateMsg {
    /// id is a human-readable name for the escrow to use later
    /// 3-20 bytes of utf-8 text
    pub id: String,
    pub client: String,
    pub cw20_whitelist: Option<Vec<String>>,
    pub work_title: String,
    pub work_desc: String,
    pub work_url: String,
    pub start_time: Option<u64>,
    pub account_min_stake_amount : u64,
    pub stake_amount: u64
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TopUpMsg {
    pub id: String,
    pub start_time: u64,
    pub end_time: u64
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConstantMsg {
    pub manager_addr: String,
    pub min_stake: String,
    pub rate_client: String,
    pub rate_manager: String,
}

impl CreateMsg {
    pub fn addr_whitelist(&self, api: &dyn Api) -> StdResult<Vec<Addr>> {
        match self.cw20_whitelist.as_ref() {
            Some(v) => v.iter().map(|h| api.addr_validate(h)).collect(),
            None => Ok(vec![]),
        }
    }
}

pub fn is_valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() < 3 || bytes.len() > 20 {
        return false;
    }
    true
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Show all open escrows. Return type is ListResponse.
    List {},
    // Details { id: String },
    DetailsAll {addr: String},
    Constants {},
    IsAdmin { addr: String},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ListResponse {
    /// list all registered ids
    pub escrows: Vec<String>,
}


#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct DetailsAllResponse {
    /// list all registered ids
    pub escrows: Vec<DetailsResponse>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct DetailsResponse {
    /// id of this escrow
    pub id: String,
    pub client: String,
    pub work_title: String,
    pub work_desc: String,
    pub work_url: String,
    pub start_time: Option<u64>,
    pub account_min_stake_amount: u64,
    pub stake_amount: u64,
    pub cw20_balance: Vec<Cw20Coin>,
    // pub account_info: Vec<AccountInfo>,
    pub account_info: String,
    pub state: u8,
    pub my_staked: String,
    pub expired: bool
   
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct IsAdminResponse {
    /// id of this escrow
    pub isadmin: bool,
}

