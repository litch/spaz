#[macro_use]
extern crate serde_json;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use anyhow::{anyhow, Error, Result};
use std::path::Path;
extern crate rand;

use cln_plugin::{Builder, Plugin};
use std::time::Duration;
use cln_rpc::{model, ClnRpc, Request};

use std::sync::{Arc, RwLock};

use tokio;
use tokio::{task, time};

use spaz::{load_configuration, Config, list_channels, randomize_fee, Amount, keysend_node, stop_handler, start_handler, list_peers, disconnect_peer, list_nodes};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {

    if let Some(plugin) = Builder::new((), tokio::io::stdin(), tokio::io::stdout())
        
        .rpcmethod("start-spazzing", "enables this plugn", start_handler)
        .rpcmethod("stop-spazzing", "disables this plugn", stop_handler)

        .start()
        .await?
    {
        let config = load_configuration(&plugin).unwrap();
        task::spawn(async move {
            loop {
                time::sleep(Duration::from_secs(
                    1
                ))
                .await;
                let config =  Config::current();
                log::info!("Spazzzzzzing - config: {:?}", config);
                match spaz_out(config.clone()).await {
                    Ok(_) => {
                        log::debug!("Success");
                    }
                    Err(err) => {
                        log::warn!("Error spazzing.  Proceeding: {:?}", err);
                    }
                };
            }
        });
        plugin.join().await
    } else {
        Ok(())
    }
}

pub async fn spaz_out(config: Arc<Config>) -> Result<(), Error> {
    if config.active == false {
        // return Ok(())
    }
    let channels = list_channels().await.unwrap();
    for channel in channels {
        let probability = 0.02;
        if rand::random::<f64>() < probability {
            match channel.short_channel_id {
                Some(id) => {
                    log::info!("Randomizing channel fee for {}", &id);
                    match randomize_fee(&id).await {
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
    let peers = list_peers().await.unwrap();
    for peer in peers {
        log::debug!("Peer under consideration: {:?}", peer);
        if peer.connected {
            let probability = 0.1; // 10% probability

            if rand::random::<f64>() < probability {
                disconnect_peer(peer.id).await.unwrap();
            }
            
        }
    }
    let nodes = list_nodes().await.unwrap();
    for node in nodes {
        log::debug!("Node under consideration: {:?}", node);
        let probability = 0.05; // 5% probability

        if rand::random::<f64>() < probability {
            match keysend_node(node.nodeid, Amount::from_msat(450)).await {
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