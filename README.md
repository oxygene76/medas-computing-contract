# MEDAS Computing Service Smart Contract

CosmWasm smart contract for the decentralized MEDAS computing marketplace. This contract coordinates provider registration, job submission, payment escrow, and automatic payment distribution.

## Overview

The MEDAS Computing Service enables a decentralized network of computing providers who offer services (like PI calculations) to clients. The smart contract acts as a trustless coordinator:

- **Providers** register their services and pricing
- **Clients** select providers and submit jobs with payment
- **Contract** holds payment in escrow and distributes it upon job completion
- **Community Pool** receives 15% of each payment automatically

## Features

- Provider registration with capabilities and pricing
- Direct provider selection by clients
- Automatic payment escrow and distribution
- Community fee (15%) and provider payment (85%)
- Job status tracking and querying
- Reputation system for providers
- Multi-service support (extensible beyond PI calculations)

## Building

### Prerequisites

- Rust 1.70+
- wasm32-unknown-unknown target
- Docker (for optimized builds)

Install Rust:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

Add wasm target:
rustup target add wasm32-unknown-unknown

### Development Build

cargo build

### Production Build (Optimized)

Compile for wasm:
cargo build --release --target wasm32-unknown-unknown

Optimize with cosmwasm optimizer:
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.12.13

Output: artifacts/medas_computing_contract.wasm

## Testing

Run all tests:
cargo test

Run with output:
cargo test -- --nocapture

## Deployment

### 1. Store Contract Code

./medasdigital-client tx wasm store artifacts/medas_computing_contract.wasm \
  --from deployer-key \
  --gas auto \
  --gas-adjustment 1.3

Note the CODE_ID from the response (e.g., CODE_ID=1)

### 2. Instantiate Contract

./medasdigital-client tx wasm instantiate <CODE_ID> \
  '{"community_pool":"medas1jv65s3grqf6v6jl3dp4t6c9t9rk99cd8zvc5e9","community_fee_percent":15}' \
  --label "MEDAS Computing Service v1.0" \
  --admin <YOUR_ADMIN_ADDRESS> \
  --from deployer-key \
  --gas auto

Save the CONTRACT_ADDRESS from the response for all future interactions.

## Usage Examples

Replace <CONTRACT_ADDRESS> with your deployed contract address in all commands below.

### Register as Provider

./medasdigital-client tx wasm execute <CONTRACT_ADDRESS> \
  '{"register_provider":{"name":"Berlin Computing Node","capabilities":[{"service_type":"pi_calculation","max_complexity":100000,"avg_completion_time":180}],"pricing":{"pi_calculation":{"base_price":"0.0001","unit":"digit"}},"endpoint":"https://berlin.medas-computing.io"}}' \
  --from provider-key \
  --gas auto

### Submit Job (Client)

./medasdigital-client tx wasm execute <CONTRACT_ADDRESS> \
  '{"submit_job":{"provider":"<PROVIDER_ADDRESS>","job_type":"pi_calculation","parameters":"{\"digits\":10000,\"method\":\"chudnovsky\"}"}}' \
  --amount 1000000umedas \
  --from client-key \
  --gas auto

### Complete Job (Provider)

./medasdigital-client tx wasm execute <CONTRACT_ADDRESS> \
  '{"complete_job":{"job_id":1,"result_hash":"abc123def456...","result_url":"https://provider.com/results/job_1.json"}}' \
  --from provider-key \
  --gas auto

## Queries

List all providers:
./medasdigital-client query wasm contract-state smart <CONTRACT_ADDRESS> '{"list_providers":{}}'

Get job status:
./medasdigital-client query wasm contract-state smart <CONTRACT_ADDRESS> '{"get_job":{"job_id":1}}'

List provider's jobs:
./medasdigital-client query wasm contract-state smart <CONTRACT_ADDRESS> '{"list_jobs_by_provider":{"provider":"<PROVIDER_ADDRESS>"}}'

## Payment Flow

1. Client submits job with payment (e.g., 1.0 MEDAS)
2. Contract holds payment in escrow
3. Provider completes job
4. Contract automatically distributes:
   - 85% (0.85 MEDAS) to Provider
   - 15% (0.15 MEDAS) to Community Pool

## Architecture

Client → Smart Contract (Escrow) → Provider Node
         ↓ (upon completion)
    Payment Distribution:
    - 85% to Provider
    - 15% to Community Pool

## Security

- Only assigned provider can complete their jobs
- Payment held in escrow (trustless)
- Admin can migrate contract (if set during instantiation)
- Blockchain-level protection against transaction replay

## License

MIT License - see LICENSE file

## Links

- MEDAS Blockchain: https://github.com/oxygene76/medasdigital2.0
- Provider Node Implementation: https://github.com/oxygene76/medasdigital-client

## Support

GitHub Issues: https://github.com/your-org/medas-computing-contract/issues
