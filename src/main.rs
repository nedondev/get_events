use ethers::{
    core::types::U64,
    providers::{Http, Middleware, Provider},
};
use std::sync::Arc;

use eyre::Result;

use env_logger::Builder;
use log::LevelFilter;

use clap::Parser;

mod events;

/// Simple program to get events from rpc with specify block and block step
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    rpc_url: String,

    #[arg(short, long)]
    target_address: String,

    /// Example: "PairCreated(address,address,address,uint256)"
    #[arg(short, long)]
    event_string: String,

    #[arg(long, default_value_t = 0)]
    start_block: u64,

    #[arg(long, default_value = None)]
    stop_block: Option<u64>,

    #[arg(long, default_value_t = 2048)]
    step_block: u64,

    #[clap(long, action)]
    block_number: bool,

    #[clap(long, action)]
    block_hash: bool,

    #[clap(long, action)]
    tx_hash: bool,

    #[arg(short, long, default_value_t = 1)]
    client_number: u64,

    #[clap(short, long, action)]
    log: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Args {
        rpc_url,
        target_address,
        event_string,
        start_block,
        stop_block,
        step_block,
        block_number,
        block_hash,
        tx_hash,
        client_number,
        log,
    } = Args::parse();

    let event_string = Arc::new(event_string);
    let target_address = Arc::new(target_address);
    if log {
        Builder::new().filter(None, LevelFilter::Info).init();
    } else {
        Builder::new().filter(None, LevelFilter::Off).init();
    }
    let provider = Provider::<Http>::try_from(rpc_url.as_str())?;
    let current_block_number: U64 = provider.get_block_number().await?;
    let to_block: u64;
    match stop_block {
        Some(stop_block) => {
            to_block = if stop_block > current_block_number.as_u64() {
                current_block_number.as_u64()
            } else {
                stop_block
            };
        }
        None => {
            to_block = current_block_number.as_u64();
        }
    }
    let mut handles = Vec::new();
    for i in 0..client_number {
        let split_start_block = start_block + (to_block - start_block) * i / (client_number);
        let split_stop_block = start_block + (to_block - start_block) * (i + 1) / (client_number);
        let provider = Provider::<Http>::try_from(rpc_url.as_str())?;
        let client = Arc::new(provider);
        let event_string = Arc::clone(&event_string);
        let target_address = Arc::clone(&target_address);
        handles.push(tokio::spawn(async move {
            // if not get block before use client to get log will got
            // `failed to lookup address information: nodename nor servname provided, or not known` error message
            match client.get_block_number().await {
                Ok(val) => {
                    log::info!("client {i} Ok {val:?}");
                }
                Err(err) => {
                    log::info!("client {i} Err {err:?}");
                }
            }
            let events = events::filter_events(
                client,
                target_address,
                event_string,
                split_start_block,
                split_stop_block,
                step_block,
            )
            .await;
            events
        }));
    }
    let mut events = Vec::new();
    for handle in handles {
        events.extend(handle.await.unwrap());
    }
    let event_strings = events::events_to_csv(
        events,
        event_string.to_string(),
        block_number,
        block_hash,
        tx_hash,
    )
    .await;
    event_strings.iter().for_each(|event| {
        println!("{event}");
    });
    Ok(())
}
