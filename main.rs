use anyhow::{Context, Result};
use tokio::time::{self, Duration, interval};
use tokio::task;
use futures::future;
use clap::Parser;
use futures::{sink::SinkExt, stream::StreamExt};
use log::{info,error};
use std::{
    env, collections::HashMap,
    time::{ Instant, SystemTime, UNIX_EPOCH},
    fs::{self, File}, path::Path,
    sync::Arc
};
use bincode;
use hex;

use reqwest::Client;

use serde_json::json;
use serde_json::Value;

use solana_client::{
    rpc_client::RpcClient,
    tpu_client::{TpuClient, TpuClientConfig}
};
use solana_sdk::{
    bs58,
    signature::{Keypair,Signature,Signer},
    pubkey::Pubkey,
    commitment_config::CommitmentConfig,
    system_program::id as system_program_id,
    message::Message,
    transaction::Transaction,
    system_instruction,
    hash::Hash,
    compute_budget::{ComputeBudgetInstruction}
};
use solana_transaction_status::UiTransactionEncoding;


use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::prelude::{
    subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequest,
    SubscribeRequestFilterSlots, SubscribeRequestPing, SubscribeUpdatePong,
    SubscribeUpdateSlot,SubscribeRequestFilterTransactions,SubscribeRequestFilterEntry,
    SubscribeRequestFilterBlocksMeta, SubscribeRequestFilterBlocks, SubscribeRequestFilterAccounts,
    SubscribeUpdateTransactionInfo
    
};
use yellowstone_grpc_proto::convert_from;

type SlotsFilterMap = HashMap<String, SubscribeRequestFilterSlots>;
type AccountFilterMap = HashMap<String, SubscribeRequestFilterAccounts>;
type TransactionsFilterMap = HashMap<String, SubscribeRequestFilterTransactions>;
type TransactionsStatusFilterMap = HashMap<String, SubscribeRequestFilterTransactions>;
type EntryFilterMap = HashMap<String, SubscribeRequestFilterEntry>;
type BlocksFilterMap = HashMap<String, SubscribeRequestFilterBlocks>;
type BlocksMetaFilterMap = HashMap<String, SubscribeRequestFilterBlocksMeta>;

use quinn::{ClientConfig, Endpoint};
use rustls::{
    ClientConfig as RustlsConfig,
    crypto::CryptoProvider,
    crypto::ring::default_provider as crypto_default_provider
};


pub static PUMPFUN_BONDINGCURVE: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
pub static RAYDIUM_PROGRAM: &str="675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
pub static SOL_MINT: &str="So11111111111111111111111111111111111111112";


#[tokio::main]
async fn main() -> Result<()> {
    let crypto_provider= crypto_default_provider();
    crypto_provider.install_default();
    
    
    dotenv::dotenv().ok();
    let http_client=Client::new();

    let JITO_TIP_ACCOUNTS: Vec<String>=vec![
        "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5".to_string(),
        "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe".to_string(),
        "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY".to_string(),
        "ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49".to_string(),
        "DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh".to_string(),
        "ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt".to_string(),
        "DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL".to_string(),
        "3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT".to_string()
    ];
    //Create web3 connection
    let rpc_url = "http://localhost:8899";
    let commitment = CommitmentConfig::processed();
    let rpc_client = RpcClient::new_with_commitment(rpc_url.to_string(),commitment);


    // //Initialize wallet from private key of .env
    // let private_key_str = env::var("PRIVATE_KEY")
    //     .context("PRIVATE_KEY environment variable not found")?;
    // let private_key_bytes = bs58::decode(private_key_str)
    //     .into_vec()
    //     .context("Failed to decode private key from base58")?;
    // let wallet = Keypair::from_bytes(&private_key_bytes)
    //     .context("Failed to create Keypair from bytes")?;
    // let public_key= wallet.pubkey();
    // println!("Public Key: {}", public_key.to_string());

    let rpc_client_arc=Arc::new(rpc_client);
    let rpc_client_tpu=Arc::clone(&rpc_client_arc);
    let rpc_client_jito=Arc::clone(&rpc_client_arc);
    // let mut tpu_client_config=TpuClientConfig::default();
    // tpu_client_config.fanout_slots=5;
    // let tpu_client=TpuClient::new(rpc_client_arc,"ws://localhost:8900", tpu_client_config)?;


    // swap_pumpfun_token_finish_jito(&rpc_client_jito, &http_client, &wallet, "6cPEZD2UK51pZpeUJCFUek4bQ28s5L1gfr4mGWrjpump", "DB52REhfC5jp7UXSLtsLFsTY5yXxnjzqNycEhkQRf6fQ", "7ChWHX4x7cDA4vgYor5T3ahcq1pBWwdFaYrT2STmExDN",1000000000, true).await;
    


    let JITO_TIP_ACCOUNTS_2=JITO_TIP_ACCOUNTS.clone();
    let other_task = tokio::spawn(async move {
        
        let mut client = GeyserGrpcClient::build_from_shared("http://localhost:10000").expect("REASON")
        .connect()
        .await;
        let mut accounts: AccountFilterMap = HashMap::new();
        let mut slots: SlotsFilterMap = HashMap::new();
        let mut transactions: TransactionsFilterMap = HashMap::new();
        let mut transactions_status: TransactionsStatusFilterMap = HashMap::new();
        let mut entries: EntryFilterMap = HashMap::new();
        let mut blocks: BlocksFilterMap = HashMap::new();
        let mut blocks_meta: BlocksMetaFilterMap = HashMap::new();
        let mut accounts_data_slice = vec![];

        transactions.insert(
            "client".to_string(),
            SubscribeRequestFilterTransactions {
                vote: Some(false),
                failed: Some(false),
                signature: None,
                account_include: vec![PUMPFUN_BONDINGCURVE.to_string()],
                account_exclude: vec![],
                account_required: vec![],
            },
        );

        //Make SubscribeRequest with properties
        let subscribe_request=SubscribeRequest {
            slots,
            accounts,
            transactions,
            transactions_status,
            entry: entries,
            blocks,
            blocks_meta,
            commitment: Some(0),
            accounts_data_slice,
            ping: None,
            from_slot: None,
        };

        //Send SubscribeRequest
        let (mut subscribe_tx, mut stream) = match client
            .expect("REASON")
            .subscribe_with_request(Some(subscribe_request))
            .await
        {
            Ok((subscribe_tx, stream)) => (subscribe_tx, stream), // Return the tuple directly
            Err(e) => {
                println!("Error while subscribing: {:?}", e);
                return; // Exit early in case of an error
            }
        };

        // Process the stream separately
        while let Some(message) = stream.next().await {
            match message {
                Ok(msg) => {
                    match msg.update_oneof {
                        Some(UpdateOneof::Account(msg)) => {
                        }
                        Some(UpdateOneof::Slot(msg)) => {
                        }
                        Some(UpdateOneof::Transaction(msg)) => {
                            let mut tx:SubscribeUpdateTransactionInfo = msg
                                .transaction
                                .ok_or_else(|| anyhow::anyhow!("no transaction in the message")).expect("REASON");
                            let mut value =match create_pretty_transaction(tx.clone()){
                                Ok(value)=>{
                                    if let Some(log_messages) = value["tx"]["meta"]["logMessages"].as_array(){
                                        let contains_initialize_mint2 = log_messages.iter().any(|entry| {
                                            entry.as_str().map_or(false, |s| s.contains("Program log: Instruction: InitializeMint2"))
                                        });
                                        let contains_buy_instruction = log_messages.iter().any(|entry| {
                                            entry.as_str().map_or(false, |s| s.contains("Program log: Instruction: Buy"))
                                        });
                                        let contains_sell_instruction = log_messages.iter().any(|entry| {
                                            entry.as_str().map_or(false, |s| s.contains("Program log: Instruction: Sell"))
                                        });
                                        if contains_initialize_mint2  {
                                            if let Some(signature) = value["signature"].as_str(){
                                                let account_keys=tx.transaction.clone().unwrap().message.unwrap().account_keys;
                                                let all_instructions=tx.transaction.clone().unwrap().message.unwrap().instructions;
                                                let encoded_keys: Vec<String> = account_keys
                                                    .iter()
                                                    .map(|key| bs58::encode(key).into_string())
                                                    .collect();
                                                let launcher=&encoded_keys[0];
                                                let mint=&encoded_keys[1];
                                                if let Some(pumpfun_program_index)=encoded_keys.iter().position(|key| key == PUMPFUN_BONDINGCURVE){
                                                    if let Some(launch_instruction)=all_instructions.iter().find(|one_instruction| { one_instruction.program_id_index==pumpfun_program_index as u32}){
                                                        let launch_account_keys: Vec<String>=launch_instruction.accounts.iter().filter_map(|&account_index| (
                                                            // encoded_keys[account_index as usize]
                                                            if (account_index as usize) < encoded_keys.len() {
                                                                Some(encoded_keys[account_index as usize].clone())
                                                            } else {
                                                                None // Skip if out of bounds
                                                            }
                                                        ).clone()).collect();
                                                        let bondingcurve=&launch_account_keys[2];
                                                        let bondingcurve_vault=&launch_account_keys[3];
                                                        let pre_balances=&value["tx"]["meta"]["preBalances"].as_array().unwrap();
                                                        let post_balances=&value["tx"]["meta"]["postBalances"].as_array().unwrap();
                                                        let pre_token_balances=&value["tx"]["meta"]["preTokenBalances"].as_array();
                                                        let post_token_balances=&value["tx"]["meta"]["postTokenBalances"].as_array().unwrap();
                                                        
                                                        let launcher_token_balance= post_token_balances.iter().find(|token_balance| {
                                                            token_balance["mint"].as_str() == Some(mint) &&
                                                            token_balance["owner"].as_str() == Some(launcher)
                                                        });
                                                        let launcher_lamports_balance_diff=(pre_balances[0].as_f64().unwrap()-post_balances[0].as_f64().unwrap());
                                                        let launcher_sol_balance_diff=(launcher_lamports_balance_diff/1_000_000_000.0).to_string();

                                                        match launcher_token_balance {
                                                            Some(data) => {
                                                                let token_balance=data["uiTokenAmount"]["uiAmount"].as_f64().unwrap();
                                                                let token_balance_amount=token_balance.to_string();
                                                                let token_balance_percent=(token_balance/1_000_000_0.0).to_string();
                                                                println!("===============NEW LAUNCH===============================");
                                                                println!("https://solscan.io/tx/{}",signature);
                                                                println!("https://photon-sol.tinyastro.io/en/lp/{}",mint);
                                                                println!("Launcher : {}",launcher);
                                                                println!("Mint : {}",mint);
                                                                println!("BondingCurve : {}",bondingcurve);
                                                                println!("Associcated BondingCurve : {}",bondingcurve_vault);
                                                                println!("Dev spent : {} SOL", launcher_sol_balance_diff);
                                                                println!("Dev token balance : {} ({} %)", token_balance_amount,token_balance_percent);
                                                                println!("========================================================");
                                                            },
                                                            None => {
                                                                println!("===============NEW LAUNCH=======================");
                                                                println!("https://solscan.io/tx/{}",signature);
                                                                println!("https://photon-sol.tinyastro.io/en/lp/{}",mint);
                                                                println!("Launcher : {}",launcher);
                                                                println!("Mint : {}",mint);
                                                                println!("BondingCurve : {}",bondingcurve);
                                                                println!("Associcated BondingCurve : {}",bondingcurve_vault);
                                                                println!("Dev spent : {} SOL", launcher_sol_balance_diff);
                                                                println!("******DEV didn't hold!*****");
                                                                println!("========================================================");
                                                            }
                                                        }
                                                        
                                                    }
                                                    println!("\n");
                                                }
                                            }
                                        }
                                        else if contains_buy_instruction || contains_sell_instruction {
                                            if let Some(signature) = value["signature"].as_str(){
                                                let account_keys=tx.transaction.clone().unwrap().message.unwrap().account_keys;
                                                let all_instructions=tx.transaction.clone().unwrap().message.unwrap().instructions;
                                                let encoded_keys: Vec<String> = account_keys
                                                    .iter()
                                                    .map(|key| bs58::encode(key).into_string())
                                                    .collect();
                                                let signer=&encoded_keys[0];
                                                let mint=&encoded_keys[1];
                                                if let Some(pumpfun_program_index)=encoded_keys.iter().position(|key| key == PUMPFUN_BONDINGCURVE){
                                                    if let Some(swap_instruction)=all_instructions.iter().find(|one_instruction| { one_instruction.program_id_index==pumpfun_program_index as u32}){
                                                        
                                                        let swap_account_keys: Vec<String>=swap_instruction.accounts.iter().filter_map(|&account_index| (
                                                            // encoded_keys[account_index as usize]
                                                            if (account_index as usize) < encoded_keys.len() {
                                                                Some(encoded_keys[account_index as usize].clone())
                                                            } else {
                                                                None // Skip if out of bounds
                                                            }
                                                        ).clone()).collect();
                                                        if swap_account_keys.len()>=5 {
                                                            let bondingcurve=&swap_account_keys[3];
                                                            let bondingcurve_vault=&swap_account_keys[4];
                                                            let pre_balances=&value["tx"]["meta"]["preBalances"].as_array().unwrap();
                                                            let post_balances=&value["tx"]["meta"]["postBalances"].as_array().unwrap();
                                                            let pre_token_balances=&value["tx"]["meta"]["preTokenBalances"].as_array();
                                                            let post_token_balances=&value["tx"]["meta"]["postTokenBalances"].as_array().unwrap();
                                                            let bondingcurve_token_balance= post_token_balances.iter().find(|token_balance| {
                                                                token_balance["mint"].as_str() != Some(SOL_MINT) &&
                                                                token_balance["owner"].as_str() == Some(bondingcurve)
                                                            });
                                                            
                                                            let bondingcurve_sol_balance= post_balances[swap_instruction.accounts[3] as usize].as_u64().unwrap();
                                                            let bondingcurve_sol_balance_sol=bondingcurve_sol_balance as f64/ 1_000_000_000.0;
                                                            if let Some(ui_token_amount)=bondingcurve_token_balance.as_ref()
                                                            .and_then(|x| x.get("uiTokenAmount"))
                                                            .and_then(|x| x.get("uiAmount"))
                                                            .and_then(|x| x.as_f64())
                                                            {
                                                                let targetToken=bondingcurve_token_balance.as_ref().and_then(|x| x.get("mint")).expect("REASON").as_str().unwrap();
                                                                // let left_tokens=bondingcurve_token_balance.as_ref().and_then(|x| x.get("uiTokenAmount")).and_then(|x| x.get("amount")).parse().unwrap();
                                                                let left_tokens = bondingcurve_token_balance
                                                                .as_ref()
                                                                .and_then(|v| v.get("uiTokenAmount"))
                                                                .and_then(|v| v.get("amount"))
                                                                .and_then(|v| v.as_str())
                                                                .and_then(|s| s.parse::<u64>().ok()).unwrap() - 206900000000000;
                                                                let bondingcurve_percent=100.0 - (((ui_token_amount - 206900000.0) * 100.0) / 793100000.0);
                                                                println!("=========PUMPFUN - SWAP======================================================");
                                                                println!("https://solscan.io/tx/{}",signature);
                                                                println!("https://photon-sol.tinyastro.io/en/lp/{}",bondingcurve);
                                                                println!("signer : {}", signer);
                                                                println!("targetToken : {}", targetToken);
                                                                println!("BondingCurve : {}",bondingcurve);
                                                                println!("Associcated BondingCurve : {}",bondingcurve_vault);
                                                                println!("{:.2} SOL in BondingCurve",bondingcurve_sol_balance_sol);
                                                                println!("{} TOKENS left in BondingCurve, ( {:.2} %)", left_tokens, bondingcurve_percent);
                                                                println!("===============================================================");
                                                                
                                                            }
                                                            else {
                                                                // println!("ERROR : could not get token balance of bondingcurve!");
                                                            }
                                                        }else {
                                                            // println!("ERROR : Not enough account keys in swap instruction");
                                                        }
                                                        
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e)=>{
                                    println!("{:?}",e);
                                }
                            };
                        }
                        Some(UpdateOneof::TransactionStatus(msg)) => {
                        }
                        Some(UpdateOneof::Entry(msg)) => {
                        }
                        Some(UpdateOneof::BlockMeta(msg)) => {
                        }
                        Some(UpdateOneof::Block(msg)) => {
                        }
                        Some(UpdateOneof::Ping(msg)) => {
                        }
                        Some(UpdateOneof::Pong(msg)) => {
                        }
                        None => {
                            println!("update not found in the message");
                            break;
                        }
                    }
                }
                Err(err) => {
                    println!("Error while receiving message: {:?}", err);
                    break; // Exit the loop on error
                }
            }
        }
    });


    let mut tasks= vec![];
    tasks.push(other_task);
    futures::future::join_all::<Vec<_>>(tasks).await;
    Ok(())
}

fn create_pretty_transaction(tx: SubscribeUpdateTransactionInfo) -> anyhow::Result<Value> {
    Ok(json!({
        "signature": Signature::try_from(tx.signature.as_slice()).context("invalid signature")?.to_string(),
        "isVote": tx.is_vote,
        "tx": convert_from::create_tx_with_meta(tx)
            .map_err(|error| anyhow::anyhow!(error))
            .context("invalid tx with meta")?
            .encode(UiTransactionEncoding::Base64, Some(u8::MAX), true)
            .context("failed to encode transaction")?,
    }))
}