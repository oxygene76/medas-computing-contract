use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Timestamp, Uint128};
use std::collections::HashMap;

#[cw_serde]
pub struct InstantiateMsg {
    pub community_pool: String,
    pub community_fee_percent: u64, // 15 = 15%
     pub default_job_timeout: u64,      
    pub heartbeat_timeout: u64,  
}

#[cw_serde]
pub enum ExecuteMsg {
    RegisterProvider {
        name: String,
        capabilities: Vec<ServiceCapability>,
        pricing: HashMap<String, PricingTier>,
        endpoint: String,
    },
    SubmitJob {
        provider: String,
        job_type: String,
        parameters: String,
    },
    CompleteJob {
        job_id: u64,
        result_hash: String,
        result_url: String,
    },
    UpdateProviderStatus {
        active: bool,
    },
    UpdateProvider {                   
        name: Option<String>,
        endpoint: Option<String>,
        pricing: Option<HashMap<String, PricingTier>>,
        capacity: Option<u32>,
    },
    HeartBeat {},                     
    FailJob {                          
        job_id: u64,
        reason: String,
    },
    CancelJob {                       
        job_id: u64,
    },
    ProcessTimedOutJobs {},            
    ProcessInactiveProviders {},       
    UpdateConfig {                     
        default_job_timeout: Option<u64>,
        heartbeat_timeout: Option<u64>,
    },
    PauseContract {},                  
    UnpauseContract {},                
}


#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    GetConfig {},
    
    #[returns(ProviderResponse)]
    GetProvider { address: String },
    
    #[returns(ProvidersResponse)]
    ListProviders {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    
    #[returns(JobResponse)]
    GetJob { job_id: u64 },
    
    #[returns(JobsResponse)]
    ListJobsByProvider {
        provider: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    
    #[returns(JobsResponse)]
    ListJobsByClient {
        client: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    
    #[returns(ProvidersResponse)]  // ADD THIS
    ListActiveProviders {},
    
    #[returns(ProviderResponse)]    // ADD THIS  
    GetProviderStats { address: String }, 
}

#[cw_serde]
pub struct ServiceCapability {
    pub service_type: String,
    pub max_complexity: u64,
    pub avg_completion_time: u64, // seconds
}

#[cw_serde]
pub struct PricingTier {
    pub base_price: Decimal,
    pub unit: String,
}

// Response types
#[cw_serde]
pub struct ConfigResponse {
    pub community_pool: String,
    pub community_fee_percent: u64,
}

#[cw_serde]
pub struct ProviderResponse {
    pub address: String,
    pub name: String,
    pub capabilities: Vec<ServiceCapability>,
    pub pricing: HashMap<String, PricingTier>,
    pub endpoint: String,
    pub capacity: u32,
    pub active_jobs: u32,
    pub total_completed: u64,
    pub reputation: Decimal,
    pub active: bool,
    pub registered_at: Timestamp,
}

#[cw_serde]
pub struct ProvidersResponse {
    pub providers: Vec<ProviderResponse>,
}

#[cw_serde]
pub struct JobResponse {
    pub id: u64,
    pub client: String,
    pub provider: String,
    pub job_type: String,
    pub parameters: String,
    pub payment_amount: Uint128,
    pub status: String,
    pub result_hash: Option<String>,
    pub result_url: Option<String>,
    pub created_at: Timestamp,
    pub completed_at: Option<Timestamp>,
}

#[cw_serde]
pub struct JobsResponse {
    pub jobs: Vec<JobResponse>,
}
#[cw_serde]
pub struct MigrateMsg {
    pub default_job_timeout: Option<u64>,  // ADD THIS
    pub heartbeat_timeout: Option<u64>,
}
