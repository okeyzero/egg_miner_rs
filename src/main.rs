use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use dotenv::dotenv;
use ethers::core::k256::ecdsa::SigningKey;
use ethers::core::rand::random;
use ethers::prelude::*;
use ethers::utils::{hex, keccak256};
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, warn};
use rayon::prelude::*;
use serde::Deserialize;
use tokio;
use tokio::time::{Duration, interval};

use crate::initialization::{log_banner, print_banner, setup_logger};

mod initialization;

static TIMES: AtomicUsize = AtomicUsize::new(0);

abigen!(
    EGG,
    r#"[
        function mine(bytes signature, address nonce, address recipient) public
        function calculateTarget() public view returns (uint256)
        function balanceOf(address account) public view returns (uint256)
        function totalSupply(uint256 id) public view returns (uint256)
    ]"#,
);
#[derive(Deserialize, Debug)]
pub struct Config {
    pub rpc_url: String,
    pub private_key: String,
    pub count: u32,
    #[serde(default = "default_prefix_gas_limit")]
    pub gas_limit: u64,
}

fn default_prefix_gas_limit() -> u64 {
    200000
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    setup_logger()?;
    print_banner();

    info!("开始执行任务");
    warn!("🐦 Twitter:[𝕏] @0xNaiXi");
    warn!("🐦 Twitter:[𝕏] @0xNaiXi");
    warn!("🐦 Twitter:[𝕏] @0xNaiXi");
    warn!("🐙 GitHub URL: https://github.com/okeyzero");
    // 解析 .env 文件
    let config = envy::from_env::<Config>()?;
    let provider = Provider::<Http>::try_from(&config.rpc_url)?;
    let chain_id = provider.get_chainid().await?;
    let private_key = config.private_key.clone();
    let wallet = private_key
        .parse::<LocalWallet>()
        .unwrap()
        .with_chain_id(chain_id.as_u64());
    let address = wallet.address();
    let nonce = provider.get_transaction_count(address, None).await?;
    info!("🏅 当前钱包地址: {:?}", address);
    info!("🏅 当前链ID: {:?}", chain_id);
    info!("🏅 钱包nonce: {:?}", nonce);

    let provider = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
    let contract_address = "0x1f7c124a63aa0dc1e046d7fea1eed9cb553072e4";
    let contract_address: Address = contract_address.parse()?;
    let contract = Arc::new(EGG::new(contract_address, provider.clone()));

    let mut success = 0;

    let speed_bar = ProgressBar::new(100);
    speed_bar.set_style(
        ProgressStyle::default_bar()
            .template("{prefix:.bold} {spinner:.green} {msg}")
            .unwrap()
            .progress_chars("##-"),
    );
    speed_bar.set_prefix("🚄 Speed");
    let mut interval = interval(Duration::from_secs(1));
    let mut max_speed = 0.0;
    tokio::spawn(async move {
        loop {
            interval.tick().await;
            let total_hash_count = TIMES.swap(0, Ordering::Relaxed);
            let hashes_per_second = total_hash_count as f64 / 1000.0;
            if hashes_per_second > max_speed {
                max_speed = hashes_per_second;
            }
            speed_bar.set_message(format!("Hash per second: {:.2} K/s - max speed: {:.2} K/s", hashes_per_second, max_speed));
        }
    });


    while success < config.count {
        log_banner(format!("第 {} 次挖矿,共 {} 次", success + 1, config.count));
        if miner(&contract, wallet.clone(), config.gas_limit).await? {
            success = success + 1;
        }
    }

    info!("🏆 任务执行完毕");

    //编译成exe 取消下面的屏蔽 不让程序关闭窗口 不然的话 会执行完任务 直接关闭窗口 无法看输出的日志了
    //tokio::time::sleep(Duration::new(1000, 0)).await;
    Ok(())
}

async fn miner(contract: &Arc<EGG<SignerMiddleware<Provider<Http>, Wallet<SigningKey>>>>, wallet: Wallet<SigningKey>, gas_limit: u64) -> Result<bool, Box<dyn std::error::Error>> {
    let balance = contract.balance_of(wallet.address()).call().await?;
    info!("🏅 balance: {:?}", balance);
    let calculate_target = contract.calculate_target().call().await?;
    info!("️🎯 Calculate target: {:?}", calculate_target);

    let nonce = mine_worker(calculate_target);

    if let Some(nonce) = nonce {
        info!("✅  Find the nonce: {:?}", nonce.address());
        // 使用私钥对 nonce 签名
        let signature = nonce.sign_message(keccak256(nonce.address().as_bytes())).await?;
        let result = contract.mine(Bytes::from(signature.to_vec()), nonce.address(), wallet.address()).gas(gas_limit).send().await.unwrap().await.unwrap();
        match result {
            Some(tx) => {
                info!("🙆 Successfully mined a block: {:?}", tx.transaction_hash);
            }
            None => {
                info!("⚠️ Failed to mine a block");
            }
        }
    } else {
        return Ok(false);
    }
    Ok(true)
}

fn mine_worker(
    target: U256,
) -> Option<Wallet<SigningKey>> {
    (0..u64::MAX)
        .into_par_iter()
        .map(|index| {
            TIMES.fetch_add(1, Ordering::Relaxed);
            // solidity 中 计算 nonce 方法为
            //bytes32 hash = keccak256(abi.encodePacked(nonce));
            //生成钱包地址
            let mut rng = rand::thread_rng(); // 为每次调用创建独立的 RNG
            let wallet = Wallet::new(&mut rng);
            let nonce = wallet.address();
            let hash = keccak256(nonce.as_bytes().to_vec());
            let hash_big_int = U256::from(&hash);
            if hash_big_int < target {
                info!("🎯 Nonce {:#?} privete {} target: {}", nonce,format!("0x{:02X?}", wallet.signer().to_bytes())
        .replace(", ", "")
        .replace("[", "")
        .replace("]", "")
        .to_lowercase(), target);
                Some(wallet)
            } else {
                None
            }
        })
        .find_any(|result| result.is_some())
        .flatten()
}
