use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Provider already registered")]
    ProviderAlreadyRegistered {},

    #[error("Provider not found")]
    ProviderNotFound {},

    #[error("Provider not active")]
    ProviderNotActive {},

    #[error("Job not found")]
    JobNotFound {},

    #[error("Invalid provider data")]
    InvalidProviderData {},

    #[error("No payment provided")]
    NoPayment {},

    #[error("Insufficient payment: expected {expected}, received {received}")]
    InsufficientPayment { expected: String, received: String },

    #[error("Invalid job parameters")]
    InvalidJobParameters {},

    #[error("Job not in correct state")]
    InvalidJobState {},
}
