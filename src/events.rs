use async_recursion::async_recursion;
use ethers::{
    core::types::{Address, Filter, Log},
    providers::{Http, Middleware, Provider},
};
use foundry_common::abi;
use std::sync::{Arc, Mutex};

use eyre::Result;
use thiserror::Error;

#[derive(Debug, Error)]
/// An error thrown when event rpc request error
pub enum EventError {
    #[error("Error from block range: {0},{1}")]
    ErrorBlockRange(u64, u64),
}

pub async fn events_to_csv(
    events: Vec<Log>,
    event_sig: String,
    is_block_number: bool,
    is_block_hash: bool,
    is_tx_hash: bool,
) -> Vec<String> {
    let mut event_string_vector: Vec<String> = Vec::new();
    let mut handles = Vec::new();
    let events = Arc::new(events);
    let event_sig = Arc::new(event_sig);
    for event_index in 0..events.len(){
        let is_block_number = Arc::new(is_block_number);
        let is_block_hash = Arc::new(is_block_hash);
        let is_tx_hash = Arc::new(is_tx_hash);
        let event_sig = Arc::clone(&event_sig);
        let events = Arc::clone(&events);
        handles.push( tokio::spawn(async move { 
        let mut token_strings: Vec<String> = Vec::new();
        // Create place holder for event 4 bytes signature.
        let mut event_string: String = "0x00000000".to_string();
        let topics = &events[event_index].topics[1..];
        topics.iter().for_each(|topic| {
            let topic = format!("{topic:#x}");
            let topic = topic.split('x').collect::<Vec<&str>>()[1];
            event_string += topic;
        });
        let data = events[event_index].data.to_string();
        let data = data.split('x').collect::<Vec<&str>>()[1];
        event_string += data;
        let tokens = abi::abi_decode(&event_sig, &event_string, true).unwrap();

        tokens.iter().for_each(|token| match token {
            ethers::abi::Token::Address(a) => {
                token_strings.push(format!("{a:#20x}"));
            }
            ethers::abi::Token::Int(i) => {
                token_strings.push(format!("{i:}"));
            }
            ethers::abi::Token::Uint(u) => {
                token_strings.push(format!("{u:}"));
            }
            _ => {
                token_strings.push(format!("{token:}"));
            }
        });
        if *is_block_number {
            let block_number = &events[event_index].block_number.unwrap();
            token_strings.push(format!("{block_number}"));
        }
        if *is_block_hash {
            let block_hash = &events[event_index].block_hash.unwrap();
            token_strings.push(format!("{block_hash:#x}"));
        }
        if *is_tx_hash {
            let tx_hash = &events[event_index].transaction_hash.unwrap();
            token_strings.push(format!("{tx_hash:#x}"));
        }
        let csv_event_field = token_strings.join(",");
            csv_event_field

        }));
    }

    for handle in handles{
        
        let csv_event_field = handle.await.unwrap();
        event_string_vector.push(format!("{csv_event_field:}"));
    }
    event_string_vector
}

#[async_recursion]
pub async fn filter_events(
    provider: Arc<Provider<Http>>,
    target_address: Arc<String>,
    event_string: Arc<String>,
    from_block: u64,
    to_block: u64,
    step: u64,
) -> Vec<Log> {
    let mut start_block = from_block;
    let mut end_block = from_block + step;
    let mut end = false;
    let mut handles = Vec::new();
    let mut log_handles =Vec::new();

    let events :Arc<Mutex<Vec<Log>>> = Arc::new(Mutex::new(Vec::new()));
    while !end {
        if start_block > to_block {
            start_block = to_block;
        }
        if end_block >= to_block {
            end_block = to_block;
            end = true;
        }
        let provider = Arc::clone(&provider);
        let target_address = Arc::clone(&target_address);
        let event_string = Arc::clone(&event_string);
        
        handles.push(tokio::spawn(filter_event(
            provider,
            target_address,
            event_string,
            start_block,
            end_block,
        )));
        start_block += step;
        end_block += step;
    }
    for handle in handles {
        let mut handles = Vec::new();
        match handle.await {
            Ok(join_ok) => match join_ok {
                // got logs
                Ok(logs) => {
                    let events = Arc::clone(&events);
                    log_handles.push(tokio::spawn(async move {
                        let mut events = events.lock().unwrap();
                        events.extend(logs);
                    }));
                }
                // got error response
                Err(err) => match err {
                    EventError::ErrorBlockRange(from_block, to_block) => {
                        log::info!("{err:?}");

                        let provider = Arc::clone(&provider);
                        let target_address = Arc::clone(&target_address);
                        let event_string = Arc::clone(&event_string);
                        handles.push(tokio::spawn(filter_events(
                            provider,
                            target_address,
                            event_string,
                            from_block,
                            to_block,
                            step,
                        )));
                    }
                },
            },
            Err(join_err) => {
                log::info!("{join_err:?}");
            }
        }
        for handle in handles {
            let events = Arc::clone(&events);
            let logs = handle.await.unwrap();
            log_handles.push(tokio::spawn(async move {
                let mut events = events.lock().unwrap();
                events.extend(logs);
            }));
        }
    }
    for handle in log_handles {
        handle.await.unwrap();
    }
    let guard = events.lock().unwrap();
    guard.clone()
}

pub async fn filter_event(
    client: Arc<Provider<Http>>,
    target_address: Arc<String>,
    event_string: Arc<String>,
    start_block: u64,
    end_block: u64,
) -> Result<Vec<Log>, EventError> {
    let filter = Filter::new()
        .address(target_address.parse::<Address>().unwrap())
        .event(&event_string)
        .from_block(start_block)
        .to_block(end_block);
    let handle = tokio::spawn(async move {
        match client.get_logs(&filter).await {
            Ok(logs)=>{
                Ok(logs)
            }
            Err(err)=>{
                 log::info!(
                     "log error block {} to {}, {:?}",
                     start_block,
                     end_block,
                     err
                 );
                Err(EventError::ErrorBlockRange(start_block, end_block))
            }
        }
    });
    handle.await.unwrap() 
}
