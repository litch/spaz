#[macro_use]
extern crate serde_json;
use anyhow::{Error, Result};
use rand::{random};

use cln_plugin::{options, Builder};
use std::time::Duration;

use std::sync::{Arc, RwLock};

use tokio;
use tokio::{task, time};

use spaz::{load_configuration, Config,  Amount, ClnClient};

pub async fn start_handler(
    config_holder: Arc<RwLock<Config>>
) -> Result<serde_json::Value, Error> {
    log::info!("Plugin start requested");
    let mut guard = config_holder.write().unwrap();
    guard.active = true;

    Ok(json!("ok"))
}

pub async fn stop_handler(
    config_holder: Arc<RwLock<Config>>
) -> Result<serde_json::Value, Error> {
    log::info!("Plugin stop requested");
    
    let mut guard = config_holder.write().unwrap();
    guard.active = false;

    Ok(json!("ok"))
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let config = Config::default();
    let config_holder = Arc::new(RwLock::new(config));
    let start_config_holder = config_holder.clone();
    let stop_config_holder = config_holder.clone();
    let loop_config_holder = config_holder.clone();
    
    if let Some(plugin) = Builder::new((), tokio::io::stdin(), tokio::io::stdout())
        .option(options::ConfigOption::new(
            "spaz-on-load",
            options::Value::Boolean(false),
            "Start spazzing on load",
        ))
        .option(options::ConfigOption::new(
            "spaz-rpc-path",
            options::Value::String("lightning-rpc".to_string()),
            "RPC path for talking to your node",
        ))
        .rpcmethod("start-spazzing", "enables this plugn", move |_p,_v| { start_handler(start_config_holder.clone()) } )
        .rpcmethod("stop-spazzing", "disables this plugn", move |_p,_v| { stop_handler(stop_config_holder.clone()) } )

        .start()
        .await?
    {
        load_configuration(&plugin, config_holder.clone()).unwrap();

        task::spawn(async move {
            loop {
                time::sleep(Duration::from_secs(
                    5
                ))
                .await;
                
                log::info!("Spazzing - config: {:?}", loop_config_holder.read().unwrap());
                match spaz_out(loop_config_holder.clone()).await {
                    Ok(_) => {
                        log::debug!("Success");
                    }
                    Err(err) => {
                        log::warn!("Error spazzing.  Continuing: {:?}", err);
                    }
                };
            }
        });
        plugin.join().await
    } else {
        Ok(())
    }
}

pub async fn maybe_randomize_channel_fee(client: Arc<ClnClient>) -> Result<(), Error> {
    let channels = client.list_channels().await?;
    for channel in channels {
        let probability = 0.02;
        if rand::random::<f64>() < probability {
            match channel.short_channel_id {
                Some(id) => {
                    log::info!("Randomizing channel fee for {}", &id);
                    match client.randomize_fee(&id).await {
                        Ok(_) => log::debug!("Successfully randomized fee"),
                        Err(e) => log::error!("Error configuring channel: {:?}", e),
                    }
                },
                None => {
                    log::debug!("No scid, so not randomizing")
                }
            }
            
        }
    }
    Ok(())
}

pub async fn maybe_disconnect_random_peer(client: Arc<ClnClient>) -> Result<(), Error> {
    let peers = client.list_peers().await?;
    for peer in peers {
        log::debug!("Peer under consideration: {:?}", peer);
        if peer.connected {
            let probability = 0.02; 

            if rand::random::<f64>() < probability {
                client.disconnect_peer(peer.id).await?;
            }
            
        }
    }
    Ok(())
}

pub async fn maybe_ping_peer_random_bytes(client: Arc<ClnClient>) -> Result<(), Error> {
    let peers = client.list_peers().await.unwrap();
    for peer in peers {
        log::debug!("Peer under consideration: {:?}", peer);
        if peer.connected {
            let probability = 0.1; // 10% probability

            if rand::random::<f64>() < probability {
                client.random_ping_peer(peer.id).await.unwrap();
            }
            
        }
    }
    Ok(())
}

pub async fn maybe_keysend_random_node(client: Arc<ClnClient>) -> Result<(), Error> {
    let nodes = client.list_nodes().await?;
    for node in nodes {
        log::debug!("Node under consideration: {:?}", node);
        let probability = 0.05; 

        if rand::random::<f64>() < probability {
            let amount: u64 = random::<u64>() % 700000 + 5000;
            match client.keysend_node(node.nodeid, Amount::from_msat(amount)).await {
                Ok(_) => {
                    log::info!("Successful keysend");
                },
                Err(err) => {
                    log::warn!("Error doing keysend: {}", err);
                }
            }
        }
        
    }
    Ok(())
}

pub async fn maybe_open_channel(client: Arc<ClnClient>) -> Result<(), Error> {
    let nodes = client.list_nodes().await?;
    for node in nodes {
        log::debug!("Perhaps open channel for node: {:?}", node);
        let probability = 0.001; 

        if rand::random::<f64>() < probability {
            
            let amount: u64 = random::<u64>() % 1000000 + 500000;
            
            match node.alias {
                Some(alias) => {
                    match client.open_channel(node.nodeid, alias, amount).await {
                        Ok(_) => {
                            log::info!("Successfully opened channel");
                        },
                        Err(err) => {
                            log::warn!("Error attempting to connect: {}", err);
                        }
                    }
                },
                None => {
                    log::debug!("Unable to try to open channel, do not have alias")
                }
            }
        
            
        }
        
    }
    Ok(())
}

pub async fn spaz_out(config_holder: Arc<RwLock<Config>>) -> Result<(), Error> {
    if config_holder.read().unwrap().active == false {
        return Ok(())
    }
    let rpc_path = config_holder.read().unwrap().rpc_path.clone();
    let client = Arc::new(ClnClient { rpc_path: rpc_path } );
    maybe_randomize_channel_fee(client.clone()).await?;
    maybe_disconnect_random_peer(client.clone()).await?;
    maybe_keysend_random_node(client.clone()).await?;
    maybe_open_channel(client.clone()).await?;
    // maybe_ping_peer_random_bytes(client.clone()).await;
    Ok(())
}
