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

use spaz::{load_configuration, Config, list_channels, stop_handler, start_handler, list_peers, disconnect_peer};

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
                log::info!("Spazzing - config: {:?}", config);
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
        return Ok(())
    }
    let channels = list_channels().await.unwrap();
    for channel in channels {
        // log::debug!("Channel under consideration: {:?}", channel);
        // match configure_channel(&channel, &config).await {
        //     Ok(_) => log::debug!("Channel successfuly configured"),
        //     Err(e) => log::error!("Error configuring channel: {:?}", e),
        // };
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
    Ok(())
}
