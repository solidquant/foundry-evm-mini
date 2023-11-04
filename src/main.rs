use anyhow::{anyhow, Result};
use ethers::abi::parse_abi;
use ethers::providers::{Middleware, Provider, Ws};
use ethers::types::{BlockNumber, H160};
use ethers_contract::BaseContract;
use revm::{
    db::CacheDB,
    primitives::{ExecutionResult, Output, TransactTo, U256 as rU256},
    EVM,
};
use std::{collections::BTreeSet, str::FromStr, sync::Arc};

use foundry_evm_mini::evm::executor::{
    fork::{BlockchainDb, BlockchainDbMeta, SharedBackend},
    inspector::{get_precompiles_for, AccessListTracer},
};

#[tokio::main]
async fn main() -> Result<()> {
    let wss_url = "ws://localhost:8546";
    let ws = Ws::connect(wss_url).await.unwrap();
    let provider = Arc::new(Provider::new(ws));

    let block = provider
        .get_block(BlockNumber::Latest)
        .await
        .unwrap()
        .unwrap();

    let shared_backend = SharedBackend::spawn_backend_thread(
        provider.clone(),
        BlockchainDb::new(
            BlockchainDbMeta {
                cfg_env: Default::default(),
                block_env: Default::default(),
                hosts: BTreeSet::from(["".to_string()]),
            },
            None,
        ),
        Some(block.number.unwrap().into()),
    );
    let db = CacheDB::new(shared_backend);

    let mut evm = EVM::new();
    evm.database(db);

    evm.env.cfg.limit_contract_code_size = Some(0x100000);
    evm.env.cfg.disable_block_gas_limit = true;
    evm.env.cfg.disable_base_fee = true;

    evm.env.block.number = rU256::from(block.number.unwrap().as_u64() + 1);

    let uniswap_v2_factory = BaseContract::from(parse_abi(&[
        "function getPair(address,address) external view returns (address)",
    ])?);

    let factory = H160::from_str("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f").unwrap();
    let weth = H160::from_str("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2").unwrap();
    let usdt = H160::from_str("0xdac17f958d2ee523a2206206994597c13d831ec7").unwrap();

    let calldata = uniswap_v2_factory.encode("getPair", (weth, usdt))?;

    evm.env.tx.caller = H160::from_str("0xD90Ed9c2679C6a9EA85A89F9af0b7b0690D530B6")
        .unwrap()
        .into();
    evm.env.tx.transact_to = TransactTo::Call(factory.into());
    evm.env.tx.data = calldata.0;
    evm.env.tx.value = rU256::ZERO;
    evm.env.tx.gas_limit = 5000000;

    let ref_tx = evm.transact_ref()?;
    let result = ref_tx.result;

    match result {
        ExecutionResult::Success { output, logs, .. } => match output {
            Output::Call(o) => {
                let pair_address: H160 = uniswap_v2_factory.decode_output("getPair", o)?;
                println!("Pair address: {:?}", pair_address);

                for log in logs {
                    println!("{:?}", log);
                }
            }
            _ => {}
        },
        _ => {}
    };

    // get access list example
    let mut access_list_inspector = AccessListTracer::new(
        Default::default(),
        evm.env.tx.caller.into(),
        factory,
        get_precompiles_for(evm.env.cfg.spec_id),
    );
    evm.inspect_ref(&mut access_list_inspector)
        .map_err(|e| anyhow!("[EVM ERROR] access list: {:?}", (e)))?;
    let access_list = access_list_inspector.access_list();
    println!("{:?}", access_list);

    Ok(())
}
