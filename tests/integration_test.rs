#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, Addr, Decimal};
    use std::collections::HashMap;

    use medas_computing_contract::contract::{execute, instantiate, query};
    use medas_computing_contract::msg::{
        ExecuteMsg, InstantiateMsg, PricingTier, QueryMsg, ServiceCapability,
    };

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            community_pool: "medas1community...".to_string(),
            community_fee_percent: 15,
        };

        let info = mock_info("creator", &coins(0, "umedas"));
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        assert_eq!(res.attributes.len(), 3);
    }

    #[test]
    fn test_register_provider() {
        let mut deps = mock_dependencies();

        // Instantiate
        let init_msg = InstantiateMsg {
            community_pool: "medas1community...".to_string(),
            community_fee_percent: 15,
        };
        instantiate(deps.as_mut(), mock_env(), mock_info("creator", &[]), init_msg).unwrap();

        // Register provider
        let mut pricing = HashMap::new();
        pricing.insert(
            "pi_calculation".to_string(),
            PricingTier {
                base_price: Decimal::percent(1), // 0.01
                unit: "digit".to_string(),
            },
        );

        let msg = ExecuteMsg::RegisterProvider {
            name: "Test Provider".to_string(),
            capabilities: vec![ServiceCapability {
                service_type: "pi_calculation".to_string(),
                max_complexity: 100000,
                avg_completion_time: 180,
            }],
            pricing,
            endpoint: "https://test.com".to_string(),
        };

        let info = mock_info("provider1", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        assert_eq!(res.attributes[0].value, "register_provider");
    }

    #[test]
    fn test_submit_and_complete_job() {
        let mut deps = mock_dependencies();

        // Setup
        let init_msg = InstantiateMsg {
            community_pool: "medas1community...".to_string(),
            community_fee_percent: 15,
        };
        instantiate(deps.as_mut(), mock_env(), mock_info("creator", &[]), init_msg).unwrap();

        // Register provider
        let mut pricing = HashMap::new();
        pricing.insert(
            "pi_calculation".to_string(),
            PricingTier {
                base_price: Decimal::percent(1),
                unit: "digit".to_string(),
            },
        );

        let register_msg = ExecuteMsg::RegisterProvider {
            name: "Test Provider".to_string(),
            capabilities: vec![ServiceCapability {
                service_type: "pi_calculation".to_string(),
                max_complexity: 100000,
                avg_completion_time: 180,
            }],
            pricing,
            endpoint: "https://test.com".to_string(),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("provider1", &[]),
            register_msg,
        )
        .unwrap();

        // Submit job
        let submit_msg = ExecuteMsg::SubmitJob {
            provider: "provider1".to_string(),
            job_type: "pi_calculation".to_string(),
            parameters: r#"{"digits":10000}"#.to_string(),
        };

        let info = mock_info("client1", &coins(1_000_000, "umedas"));
        let res = execute(deps.as_mut(), mock_env(), info, submit_msg).unwrap();

        let job_id = res
            .attributes
            .iter()
            .find(|attr| attr.key == "job_id")
            .unwrap()
            .value
            .parse::<u64>()
            .unwrap();

        // Complete job
        let complete_msg = ExecuteMsg::CompleteJob {
            job_id,
            result_hash: "abc123".to_string(),
            result_url: "https://test.com/result".to_string(),
        };

        let info = mock_info("provider1", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, complete_msg).unwrap();

        assert_eq!(res.messages.len(), 2); // Community + Provider payment
    }

    #[test]
    fn test_query_providers() {
        let mut deps = mock_dependencies();

        // Setup
        let init_msg = InstantiateMsg {
            community_pool: "medas1community...".to_string(),
            community_fee_percent: 15,
        };
        instantiate(deps.as_mut(), mock_env(), mock_info("creator", &[]), init_msg).unwrap();

        // Register provider
        let mut pricing = HashMap::new();
        pricing.insert(
            "pi_calculation".to_string(),
            PricingTier {
                base_price: Decimal::percent(1),
                unit: "digit".to_string(),
            },
        );

        let register_msg = ExecuteMsg::RegisterProvider {
            name: "Test Provider".to_string(),
            capabilities: vec![ServiceCapability {
                service_type: "pi_calculation".to_string(),
                max_complexity: 100000,
                avg_completion_time: 180,
            }],
            pricing,
            endpoint: "https://test.com".to_string(),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("provider1", &[]),
            register_msg,
        )
        .unwrap();

        // Query providers
        let query_msg = QueryMsg::ListProviders {
            start_after: None,
            limit: None,
        };

        let res = query(deps.as_ref(), mock_env(), query_msg).unwrap();
        println!("Providers response: {:?}", res);
    }
}
