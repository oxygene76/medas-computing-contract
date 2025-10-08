use cosmwasm_std::{Addr, Decimal, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::msg::{PricingTier, ServiceCapability};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub community_pool: Addr,
    pub community_fee_percent: u64,
    pub default_job_timeout: u64,      
    pub heartbeat_timeout: u64,        
    pub paused: bool,                  
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Provider {
    pub address: Addr,
    pub name: String,
    pub capabilities: Vec<ServiceCapability>,
    pub pricing: HashMap<String, PricingTier>,
    pub endpoint: String,
    pub capacity: u32,
    pub active_jobs: u32,
    pub total_completed: u64,
    pub total_failed: u64,
    pub reputation: Decimal,
    pub active: bool,
    pub registered_at: Timestamp,
    pub last_heartbeat: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Job {
    pub id: u64,
    pub client: Addr,
    pub provider: Addr,
    pub job_type: String,
    pub parameters: String,
    pub payment_amount: Uint128,
    pub status: JobStatus,
    pub result_hash: Option<String>,
    pub result_url: Option<String>,
    pub created_at: Timestamp,
    pub completed_at: Option<Timestamp>,
    pub deadline: u64,                 
    pub failure_reason: Option<String>, 
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum JobStatus {
    Submitted,
    Processing,
    Completed,
    Failed,
    Cancelled, 
}
impl JobStatus {
    pub fn to_string(&self) -> String {
        match self {
            JobStatus::Submitted => "submitted".to_string(),
            JobStatus::Processing => "processing".to_string(),
            JobStatus::Completed => "completed".to_string(),
            JobStatus::Failed => "failed".to_string(),
            JobStatus::Cancelled => "cancelled".to_string(),  // Add this
        }
    }
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const PROVIDERS: Map<&Addr, Provider> = Map::new("providers");
pub const JOBS: Map<u64, Job> = Map::new("jobs");
pub const NEXT_JOB_ID: Item<u64> = Item::new("next_job_id");
pub const JOBS_BY_PROVIDER: Map<(&Addr, u64), ()> = Map::new("jobs_by_provider");
pub const JOBS_BY_CLIENT: Map<(&Addr, u64), ()> = Map::new("jobs_by_client");
