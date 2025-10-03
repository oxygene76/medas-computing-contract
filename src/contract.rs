use cosmwasm_std::{
    entry_point, to_json_binary, Addr, BankMsg, Binary, Coin, Decimal, Deps, DepsMut, Env,
    MessageInfo, Order, Response, StdResult, Uint128,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, JobResponse, JobsResponse, ProviderResponse,
    ProvidersResponse, QueryMsg,
};
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
    match msg {
        ExecuteMsg::RegisterProvider {
            name,
            capabilities,
            pricing,
            endpoint,
        } => execute_register_provider(deps, env, info, name, capabilities, pricing, endpoint),
        ExecuteMsg::SubmitJob {
            provider,
            job_type,
            parameters,
        } => execute_submit_job(deps, env, info, provider, job_type, parameters),
        ExecuteMsg::CompleteJob {
            job_id,
            result_hash,
            result_url,
        } => execute_complete_job(deps, env, info, job_id, result_hash, result_url),
        ExecuteMsg::UpdateProviderStatus { active } => {
            execute_update_provider_status(deps, info, active)
        }
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
    let provider_fee = job.payment_amount.checked_sub(community_fee)?;

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

    let start = start_after.map(|s| {
    let addr = deps.api.addr_validate(&s).unwrap();
    Bound::exclusive(&addr)
    });

    let providers: Vec<ProviderResponse> = PROVIDERS
        .range(deps.storage, start, None, Order::Ascending)
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
        .collect::<StdResult<Vec<_>>>()?;

    Ok(ProvidersResponse { providers })
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
