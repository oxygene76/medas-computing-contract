use cosmwasm_std::{
    entry_point, to_json_binary, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps, 
    DepsMut, Env, MessageInfo, Order, Response, StdResult,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;
use std::collections::HashMap;  // ADD THIS

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, JobResponse, JobsResponse, 
    MigrateMsg, PricingTier, ProviderResponse, ProvidersResponse, QueryMsg};  // ADD PricingTier

use crate::state::{
    Config, Job, JobStatus, Provider, CONFIG, JOBS, JOBS_BY_CLIENT, JOBS_BY_PROVIDER,
    NEXT_JOB_ID, PROVIDERS,
};

const CONTRACT_NAME: &str = "crates.io:medas-computing-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let community_pool = deps.api.addr_validate(&msg.community_pool)?;

   let config = Config {
    community_pool,
    community_fee_percent: msg.community_fee_percent,
    default_job_timeout: msg.default_job_timeout,      
    heartbeat_timeout: msg.heartbeat_timeout,          
    paused: false,                                    
    };
    CONFIG.save(deps.storage, &config)?;
    NEXT_JOB_ID.save(deps.storage, &1u64)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("community_pool", msg.community_pool)
        .add_attribute("community_fee_percent", msg.community_fee_percent.to_string()))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    // Check if contract is paused (except for unpause)
    let config = CONFIG.load(deps.storage)?;
    if config.paused && !matches!(msg, ExecuteMsg::UnpauseContract {}) {
        return Err(ContractError::ContractPaused {});
    }
    
    match msg {
        ExecuteMsg::RegisterProvider { name, capabilities, pricing, endpoint } => 
            execute_register_provider(deps, env, info, name, capabilities, pricing, endpoint),
        ExecuteMsg::SubmitJob { provider, job_type, parameters } => 
            execute_submit_job(deps, env, info, provider, job_type, parameters),
        ExecuteMsg::CompleteJob { job_id, result_hash, result_url } => 
            execute_complete_job(deps, env, info, job_id, result_hash, result_url),
        ExecuteMsg::UpdateProviderStatus { active } => 
            execute_update_provider_status(deps, info, active),
        ExecuteMsg::HeartBeat {} => 
            execute_heartbeat(deps, env, info),
        ExecuteMsg::UpdateProvider { name, endpoint, pricing, capacity } => 
            execute_update_provider(deps, env, info, name, endpoint, pricing, capacity),
        ExecuteMsg::FailJob { job_id, reason } => 
            execute_fail_job(deps, env, info, job_id, reason),
        ExecuteMsg::CancelJob { job_id } => 
            execute_cancel_job(deps, env, info, job_id),
        ExecuteMsg::ProcessTimedOutJobs {} => 
            execute_process_timed_out_jobs(deps, env, info),
        ExecuteMsg::ProcessInactiveProviders {} => 
            execute_process_inactive_providers(deps, env, info),
        ExecuteMsg::UpdateConfig { default_job_timeout, heartbeat_timeout } => 
            execute_update_config(deps, info, default_job_timeout, heartbeat_timeout),
        ExecuteMsg::PauseContract {} => 
            execute_pause_contract(deps, info),
        ExecuteMsg::UnpauseContract {} => 
            execute_unpause_contract(deps, info),
    }
}

pub fn execute_register_provider(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    name: String,
    capabilities: Vec<crate::msg::ServiceCapability>,
    pricing: std::collections::HashMap<String, crate::msg::PricingTier>,
    endpoint: String,
) -> Result<Response, ContractError> {
    // Check if already registered
    if PROVIDERS.has(deps.storage, &info.sender) {
        return Err(ContractError::ProviderAlreadyRegistered {});
    }

    // Validate data
    if name.is_empty() || capabilities.is_empty() {
        return Err(ContractError::InvalidProviderData {});
    }

    let provider = Provider {
        address: info.sender.clone(),
        name: name.clone(),
        capabilities,
        pricing,
        endpoint,
        capacity: 10,
        active_jobs: 0,
        total_completed: 0,
        total_failed: 0,
        reputation: Decimal::percent(50),
        active: true,
        registered_at: env.block.time,
        last_heartbeat: env.block.time.seconds(), 
    };

    PROVIDERS.save(deps.storage, &info.sender, &provider)?;

    Ok(Response::new()
        .add_attribute("action", "register_provider")
        .add_attribute("provider", info.sender.to_string())
        .add_attribute("name", name))
}

pub fn execute_submit_job(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    provider_addr: String,
    job_type: String,
    parameters: String,
) -> Result<Response, ContractError> {
    let provider = deps.api.addr_validate(&provider_addr)?;

    // Check if provider exists and is active
    let mut provider_info = PROVIDERS
        .load(deps.storage, &provider)
        .map_err(|_| ContractError::ProviderNotFound {})?;

    if !provider_info.active {
        return Err(ContractError::ProviderNotActive {});
    }

    // Extract payment
    let payment = info
        .funds
        .iter()
        .find(|c| c.denom == "umedas")
        .ok_or(ContractError::NoPayment {})?;

    if payment.amount.is_zero() {
        return Err(ContractError::NoPayment {});
    }

    // Load config for timeout - ADD THIS LINE!
    let config = CONFIG.load(deps.storage)?;

    // Create job
    let job_id = NEXT_JOB_ID.update(deps.storage, |id| -> StdResult<_> { Ok(id + 1) })?;

    let job = Job {
        id: job_id,
        client: info.sender.clone(),
        provider: provider.clone(),
        job_type: job_type.clone(),
        parameters: parameters.clone(),
        payment_amount: payment.amount,
        status: JobStatus::Submitted,
        result_hash: None,
        result_url: None,
        created_at: env.block.time,
        completed_at: None,
        deadline: env.block.time.seconds() + config.default_job_timeout,  
        failure_reason: None,             
    };

    JOBS.save(deps.storage, job_id, &job)?;

    // Update indices
    JOBS_BY_PROVIDER.save(deps.storage, (&provider, job_id), &())?;
    JOBS_BY_CLIENT.save(deps.storage, (&info.sender, job_id), &())?;

    // Update provider active jobs
    provider_info.active_jobs += 1;
    PROVIDERS.save(deps.storage, &provider, &provider_info)?;

    Ok(Response::new()
        .add_attribute("action", "submit_job")
        .add_attribute("job_id", job_id.to_string())
        .add_attribute("provider", provider.to_string())
        .add_attribute("client", info.sender.to_string())
        .add_attribute("payment", payment.amount.to_string()))
}

pub fn execute_complete_job(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    job_id: u64,
    result_hash: String,
    result_url: String,
) -> Result<Response, ContractError> {
    let mut job = JOBS
        .load(deps.storage, job_id)
        .map_err(|_| ContractError::JobNotFound {})?;

    // Only assigned provider can complete
    if job.provider != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    // Check job status
    if job.status != JobStatus::Submitted && job.status != JobStatus::Processing {
        return Err(ContractError::InvalidJobState {});
    }

    // Update job
    job.status = JobStatus::Completed;
    job.result_hash = Some(result_hash);
    job.result_url = Some(result_url);
    job.completed_at = Some(env.block.time);

    JOBS.save(deps.storage, job_id, &job)?;

    // Update provider stats
    let mut provider = PROVIDERS.load(deps.storage, &job.provider)?;
    provider.active_jobs = provider.active_jobs.saturating_sub(1);
    provider.total_completed += 1;
    PROVIDERS.save(deps.storage, &job.provider, &provider)?;

    // Calculate and distribute payment
    let config = CONFIG.load(deps.storage)?;
    let community_fee = job.payment_amount * Decimal::percent(config.community_fee_percent);
    let provider_fee = job.payment_amount.checked_sub(community_fee)
    .map_err(|e| ContractError::Std(cosmwasm_std::StdError::generic_err(e.to_string())))?;

    let mut messages = vec![];

    // Send to community pool
    if !community_fee.is_zero() {
        messages.push(BankMsg::Send {
            to_address: config.community_pool.to_string(),
            amount: vec![Coin {
                denom: "umedas".to_string(),
                amount: community_fee,
            }],
        });
    }

    // Send to provider
    messages.push(BankMsg::Send {
        to_address: job.provider.to_string(),
        amount: vec![Coin {
            denom: "umedas".to_string(),
            amount: provider_fee,
        }],
    });

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "complete_job")
        .add_attribute("job_id", job_id.to_string())
        .add_attribute("provider_payment", provider_fee.to_string())
        .add_attribute("community_fee", community_fee.to_string()))
}

pub fn execute_update_provider_status(
    deps: DepsMut,
    info: MessageInfo,
    active: bool,
) -> Result<Response, ContractError> {
    let mut provider = PROVIDERS
        .load(deps.storage, &info.sender)
        .map_err(|_| ContractError::ProviderNotFound {})?;

    provider.active = active;
    PROVIDERS.save(deps.storage, &info.sender, &provider)?;

    Ok(Response::new()
        .add_attribute("action", "update_provider_status")
        .add_attribute("provider", info.sender.to_string())
        .add_attribute("active", active.to_string()))
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_json_binary(&query_config(deps)?),
        QueryMsg::GetProvider { address } => to_json_binary(&query_provider(deps, address)?),
        QueryMsg::ListProviders { start_after, limit } => {
            to_json_binary(&query_list_providers(deps, start_after, limit)?)
        }
        QueryMsg::GetJob { job_id } => to_json_binary(&query_job(deps, job_id)?),
        QueryMsg::ListJobsByProvider {
            provider,
            start_after,
            limit,
        } => to_json_binary(&query_jobs_by_provider(deps, provider, start_after, limit)?),
        QueryMsg::ListJobsByClient {
            client,
            start_after,
            limit,
        } => to_json_binary(&query_jobs_by_client(deps, client, start_after, limit)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        community_pool: config.community_pool.to_string(),
        community_fee_percent: config.community_fee_percent,
    })
}

fn query_provider(deps: Deps, address: String) -> StdResult<ProviderResponse> {
    let addr = deps.api.addr_validate(&address)?;
    let provider = PROVIDERS.load(deps.storage, &addr)?;

    Ok(ProviderResponse {
        address: provider.address.to_string(),
        name: provider.name,
        capabilities: provider.capabilities,
        pricing: provider.pricing,
        endpoint: provider.endpoint,
        capacity: provider.capacity,
        active_jobs: provider.active_jobs,
        total_completed: provider.total_completed,
        reputation: provider.reputation,
        active: provider.active,
        registered_at: provider.registered_at,
    })
}

fn query_list_providers(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<ProvidersResponse> {
    let limit = limit.unwrap_or(50).min(100) as usize;

    let providers: StdResult<Vec<ProviderResponse>> = if let Some(start_addr_str) = start_after {
        let start_addr = deps.api.addr_validate(&start_addr_str)?;
        PROVIDERS
            .range(deps.storage, Some(Bound::exclusive(&start_addr)), None, Order::Ascending)
            .take(limit)
            .map(|item| {
                let (_, provider) = item?;
                Ok(ProviderResponse {
                    address: provider.address.to_string(),
                    name: provider.name,
                    capabilities: provider.capabilities,
                    pricing: provider.pricing,
                    endpoint: provider.endpoint,
                    capacity: provider.capacity,
                    active_jobs: provider.active_jobs,
                    total_completed: provider.total_completed,
                    reputation: provider.reputation,
                    active: provider.active,
                    registered_at: provider.registered_at,
                })
            })
            .collect()
    } else {
        PROVIDERS
            .range(deps.storage, None, None, Order::Ascending)
            .take(limit)
            .map(|item| {
                let (_, provider) = item?;
                Ok(ProviderResponse {
                    address: provider.address.to_string(),
                    name: provider.name,
                    capabilities: provider.capabilities,
                    pricing: provider.pricing,
                    endpoint: provider.endpoint,
                    capacity: provider.capacity,
                    active_jobs: provider.active_jobs,
                    total_completed: provider.total_completed,
                    reputation: provider.reputation,
                    active: provider.active,
                    registered_at: provider.registered_at,
                })
            })
            .collect()
    };

    Ok(ProvidersResponse { providers: providers? })
}
fn query_job(deps: Deps, job_id: u64) -> StdResult<JobResponse> {
    let job = JOBS.load(deps.storage, job_id)?;

    Ok(JobResponse {
        id: job.id,
        client: job.client.to_string(),
        provider: job.provider.to_string(),
        job_type: job.job_type,
        parameters: job.parameters,
        payment_amount: job.payment_amount,
        status: job.status.to_string(),
        result_hash: job.result_hash,
        result_url: job.result_url,
        created_at: job.created_at,
        completed_at: job.completed_at,
    })
}

fn query_jobs_by_provider(
    deps: Deps,
    provider: String,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<JobsResponse> {
    let provider_addr = deps.api.addr_validate(&provider)?;
    let limit = limit.unwrap_or(10).min(50) as usize;

    let start = start_after.map(|id| Bound::exclusive(id));

    let job_ids: Vec<u64> = JOBS_BY_PROVIDER
        .prefix(&provider_addr)
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;

    let jobs: Vec<JobResponse> = job_ids
        .into_iter()
        .map(|job_id| query_job(deps, job_id))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(JobsResponse { jobs })
}

fn query_jobs_by_client(
    deps: Deps,
    client: String,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<JobsResponse> {
    let client_addr = deps.api.addr_validate(&client)?;
    let limit = limit.unwrap_or(10).min(50) as usize;

    let start = start_after.map(|id| Bound::exclusive(id));

    let job_ids: Vec<u64> = JOBS_BY_CLIENT
        .prefix(&client_addr)
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;

    let jobs: Vec<JobResponse> = job_ids
        .into_iter()
        .map(|job_id| query_job(deps, job_id))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(JobsResponse { jobs })
}
/// Heartbeat handler - providers send regular heartbeats to indicate they are online
/// This updates the provider's last_heartbeat timestamp and sets them as active
pub fn execute_heartbeat(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // Update provider's heartbeat timestamp
    PROVIDERS.update(deps.storage, &info.sender, |provider| -> Result<_, ContractError> {
        let mut p = provider.ok_or(ContractError::ProviderNotFound {})?;
        p.last_heartbeat = env.block.time.seconds();
        p.active = true;
        Ok(p)
    })?;
    
    Ok(Response::new()
        .add_attribute("action", "heartbeat")
        .add_attribute("provider", info.sender.to_string())
        .add_attribute("timestamp", env.block.time.seconds().to_string()))
}

/// Update provider information - allows providers to modify their settings
/// Can update name, endpoint, pricing, and capacity
pub fn execute_update_provider(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    name: Option<String>,
    endpoint: Option<String>,
    pricing: Option<HashMap<String, PricingTier>>,
    capacity: Option<u32>,
) -> Result<Response, ContractError> {
    // Load and update provider information
    PROVIDERS.update(deps.storage, &info.sender, |provider| -> Result<_, ContractError> {
        let mut p = provider.ok_or(ContractError::ProviderNotFound {})?;
        
        // Update fields if provided
        if let Some(n) = name {
            p.name = n;
        }
        if let Some(e) = endpoint {
            p.endpoint = e;
        }
        if let Some(pr) = pricing {
            p.pricing = pr;
        }
        if let Some(c) = capacity {
            p.capacity = c;
        }
        
        Ok(p)
    })?;
    
    Ok(Response::new()
        .add_attribute("action", "update_provider")
        .add_attribute("provider", info.sender.to_string()))
}

/// Fail a job - provider marks job as failed and client receives full refund
/// Only the assigned provider can fail their own jobs
pub fn execute_fail_job(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    job_id: u64,
    reason: String,
) -> Result<Response, ContractError> {
    // Load job
    let mut job = JOBS.load(deps.storage, job_id)?;
    
    // Only the assigned provider can fail the job
    if info.sender != job.provider {
        return Err(ContractError::Unauthorized {});
    }
    
    // Job must be in submitted state
    if job.status != JobStatus::Submitted {
        return Err(ContractError::InvalidJobState {});  // ← Verwendet bestehenden Error
    }
    
    // Update job status
    job.status = JobStatus::Failed;
    job.failure_reason = Some(reason.clone());
    job.completed_at = Some(env.block.time);
    JOBS.save(deps.storage, job_id, &job)?;
    
    // Update provider statistics
    let mut provider = PROVIDERS.load(deps.storage, &job.provider)?;
    provider.active_jobs = provider.active_jobs.saturating_sub(1);
    provider.total_failed = provider.total_failed.saturating_add(1);
    provider.reputation = calculate_reputation(&provider);
    PROVIDERS.save(deps.storage, &job.provider, &provider)?;
    
    // Refund full payment to client
    let refund_msg = BankMsg::Send {
    to_address: job.client.to_string(),
    amount: vec![Coin {
        denom: "umedas".to_string(),
        amount: job.payment_amount,
    }],
};
    
    Ok(Response::new()
        .add_message(refund_msg)
        .add_attribute("action", "fail_job")
        .add_attribute("job_id", job_id.to_string())
        .add_attribute("reason", reason)
        .add_attribute("refund_amount", job.payment_amount.to_string())) 
}

/// Cancel a job - client can cancel within 5 minutes and receive full refund
/// Only the client who submitted the job can cancel it
pub fn execute_cancel_job(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    job_id: u64,
) -> Result<Response, ContractError> {
    // Load job
    let mut job = JOBS.load(deps.storage, job_id)?;
    
    // Only the client can cancel their job
    if info.sender != job.client {
        return Err(ContractError::Unauthorized {});
    }
    
    // Job must be in submitted state
    if job.status != JobStatus::Submitted {
        return Err(ContractError::InvalidJobState {});  // ← Verwendet bestehenden Error
    }
    
    // Check if within 5-minute cancellation window
    let time_elapsed = env.block.time.seconds() - job.created_at.seconds();
    if time_elapsed > 300 {  // 300 seconds = 5 minutes
        return Err(ContractError::CancelWindowExpired {});
    }
    
    // Update job status
    job.status = JobStatus::Cancelled;
    job.completed_at = Some(env.block.time);
    JOBS.save(deps.storage, job_id, &job)?;
    
    // Update provider statistics (no reputation penalty for cancellation)
    let mut provider = PROVIDERS.load(deps.storage, &job.provider)?;
    provider.active_jobs = provider.active_jobs.saturating_sub(1);
    PROVIDERS.save(deps.storage, &job.provider, &provider)?;
    
    // Refund full payment to client
    let refund_msg = BankMsg::Send {
    to_address: job.client.to_string(),
    amount: vec![Coin {
        denom: "umedas".to_string(),
        amount: job.payment_amount,
    }],
    };
    
    Ok(Response::new()
        .add_message(refund_msg)
        .add_attribute("action", "cancel_job")
        .add_attribute("job_id", job_id.to_string())
        .add_attribute("refund_amount", job.payment_amount.to_string()))
}

/// Process timed out jobs - automatically fails and refunds jobs that exceeded their deadline
/// Can be called by anyone to clean up expired jobs
pub fn execute_process_timed_out_jobs(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    let current_time = env.block.time.seconds();
    let mut messages: Vec<CosmosMsg> = vec![];
    let mut processed_jobs = vec![];
    
    // Iterate through all jobs to find timed out ones
    let jobs: Vec<_> = JOBS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    
    for (job_id, mut job) in jobs {
        // Only process submitted jobs
        if job.status != JobStatus::Submitted {
            continue;
        }
        
        // Check if job has exceeded its deadline
        if current_time > job.deadline {
            // Mark job as failed
            job.status = JobStatus::Failed;
            job.failure_reason = Some("Timeout: Job not completed within deadline".to_string());
            job.completed_at = Some(env.block.time);
            JOBS.save(deps.storage, job_id, &job)?;
            
            // Update provider statistics (timeout counts as failure)
            let mut provider = PROVIDERS.load(deps.storage, &job.provider)?;
            provider.active_jobs = provider.active_jobs.saturating_sub(1);
            provider.total_failed = provider.total_failed.saturating_add(1);
            provider.reputation = calculate_reputation(&provider);
            PROVIDERS.save(deps.storage, &job.provider, &provider)?;
            
            // Prepare refund message
            messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: job.client.to_string(),
            amount: vec![Coin {
            denom: "umedas".to_string(),
            amount: job.payment_amount,
            }],
            }));
            
            processed_jobs.push(job_id);
        }
    }
    
    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "process_timed_out_jobs")
        .add_attribute("processed_count", processed_jobs.len().to_string())
        .add_attribute("job_ids", format!("{:?}", processed_jobs)))
}

/// Process inactive providers - deactivates providers that haven't sent heartbeat
/// Can be called by anyone to clean up inactive providers
pub fn execute_process_inactive_providers(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let current_time = env.block.time.seconds();
    let mut deactivated = vec![];
    
    // Iterate through all providers
    let providers: Vec<_> = PROVIDERS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    
    for (addr, mut provider) in providers {
        if provider.active {
            // Check time since last heartbeat
            let time_since_heartbeat = current_time - provider.last_heartbeat;
            
            // Deactivate if exceeded timeout threshold
            if time_since_heartbeat > config.heartbeat_timeout {
                provider.active = false;
                PROVIDERS.save(deps.storage, &addr, &provider)?;
                deactivated.push(addr.to_string());
            }
        }
    }
    
    Ok(Response::new()
        .add_attribute("action", "process_inactive_providers")
        .add_attribute("deactivated_count", deactivated.len().to_string())
        .add_attribute("providers", deactivated.join(",")))
}

/// Update contract configuration - admin only
/// Can update job timeout and heartbeat timeout settings
pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    default_job_timeout: Option<u64>,
    heartbeat_timeout: Option<u64>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    
    // TODO: Add admin check
    // if info.sender != config.admin {
    //     return Err(ContractError::Unauthorized {});
    // }
    
    // Update config fields if provided
    if let Some(timeout) = default_job_timeout {
        config.default_job_timeout = timeout;
    }
    if let Some(hb_timeout) = heartbeat_timeout {
        config.heartbeat_timeout = hb_timeout;
    }
    
    CONFIG.save(deps.storage, &config)?;
    
    Ok(Response::new()
        .add_attribute("action", "update_config")
        .add_attribute("default_job_timeout", config.default_job_timeout.to_string())
        .add_attribute("heartbeat_timeout", config.heartbeat_timeout.to_string()))
}

/// Pause contract - emergency pause to stop all operations
/// Admin only - useful in case of critical issues
pub fn execute_pause_contract(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    
    // TODO: Add admin check
    // if info.sender != config.admin {
    //     return Err(ContractError::Unauthorized {});
    // }
    
    config.paused = true;
    CONFIG.save(deps.storage, &config)?;
    
    Ok(Response::new()
        .add_attribute("action", "pause_contract")
        .add_attribute("paused", "true"))
}

/// Unpause contract - resume operations after emergency pause
/// Admin only
pub fn execute_unpause_contract(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    
    // TODO: Add admin check
    // if info.sender != config.admin {
    //     return Err(ContractError::Unauthorized {});
    // }
    
    config.paused = false;
    CONFIG.save(deps.storage, &config)?;
    
    Ok(Response::new()
        .add_attribute("action", "unpause_contract")
        .add_attribute("paused", "false"))
}

/// Calculate provider reputation based on success rate
/// Returns a decimal percentage (0-100)
fn calculate_reputation(provider: &Provider) -> Decimal {
    let total = provider.total_completed + provider.total_failed;
    
    // Return 100% if no jobs completed yet
    if total == 0 {
        return Decimal::percent(100);
    }
    
    // Calculate success rate as percentage
    let success_rate = provider.total_completed as f64 / total as f64;
    Decimal::from_ratio((success_rate * 100.0) as u128, 1u128)
}
#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    // Update config with new timeout values if provided
    let mut config = CONFIG.load(deps.storage)?;
    
    if let Some(timeout) = msg.default_job_timeout {
        config.default_job_timeout = timeout;
    }
    if let Some(hb_timeout) = msg.heartbeat_timeout {
        config.heartbeat_timeout = hb_timeout;
    }
    
    CONFIG.save(deps.storage, &config)?;
    
    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", CONTRACT_VERSION)
        .add_attribute("to_version", env!("CARGO_PKG_VERSION")))
}
