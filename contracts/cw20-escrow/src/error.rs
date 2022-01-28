use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Work is not started yet")]
    NotStarted {},

    #[error("You are not creator")]
    NotClient {},

    #[error("You are not manager")]
    NotManager {},

    #[error("No rewards left")]
    NotLeft {},

    #[error("Did not got all rewards")]
    NotFinished {},

    #[error("Work is already started")]
    AlreadyStarted {},

    #[error("Work is not expired yet")]
    WorkNotExpired {},

    #[error("No accounts staked")]
    NobodyStaked {},

    #[error("You did not stake")]
    DidntStaked {},

    #[error("Only accepts tokens in the cw20_whitelist")]
    NotInWhitelist {},

    #[error("Work is expired")]
    Expired {},

    

    #[error("Still in your staking expired")]
    AccountNotExpired {},

    #[error("Send some coins to create an escrow")]
    EmptyBalance {},

    #[error("Stake is ended")]
    StakeFinished {},

    #[error("Escrow id already in use")]
    AlreadyInUse {},

    #[error("Insufficient token amount for create work.")]
    InsufficientCreate {},

    #[error("Insufficient token amount for stake.")]
    InsufficientTopUp {},
}
