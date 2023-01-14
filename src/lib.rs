extern crate serde_json;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use anyhow::{anyhow, Error, Result};
use std::{path::Path, sync::{RwLock}};
extern crate rand;
use rand::random;

use cln_plugin::{Plugin, options};

use cln_rpc::{model::{self}, ClnRpc, Request};

use std::sync::{Arc};

pub struct ClnClient {
    pub rpc_path: String
}

impl ClnClient {
    async fn call(&self, request: Request) -> core::result::Result<String, Error> {
        // let config = self.config.read().unwrap();
        
        let config_path = &self.rpc_path;
        let path = Path::new(config_path);

        let mut rpc = match ClnRpc::new(path).await {
            Ok(c) => c,
            Err(e) => {
                log::error!("Error initializing CLN RPC - does path {} exist {}", &path.to_string_lossy(), e);
                return Err(e)
            }
        };
        let response = rpc
            .call(request.clone())
            .await
            .map_err(|e| anyhow!("Error calling {:?}: {:?}", request, e))?;
    
        Ok(serde_json::to_string_pretty(&response)?)
    }    

    pub async fn disconnect_peer(&self, pubkey: cln_rpc::primitives::PublicKey) -> Result<(), Error> {
        log::info!("Disconnecting from peer: {:?}", pubkey);
        let req = Request::Disconnect(model::DisconnectRequest { id: pubkey, force: Some(true) });
        let _res = self.call(req).await?;
        Ok(())
    }

    pub async fn list_channels(&self) -> Result<Vec<Channel>, Error> {
        let req = Request::ListFunds(model::ListfundsRequest { spent: Some(false)} );
        let res = self.call(req).await?;
        log::trace!("{}", &res);
    
        let de: ListFundsResponse = serde_json::from_str(&res).unwrap();
        Ok(de.result.channels)
    } 

    pub async fn list_peers(&self) -> Result<Vec<Peer>, Error> {
        let req = Request::ListPeers(model::ListpeersRequest { id: None, level: None });
        let res = self.call(req).await?;
        log::trace!("{}", &res);
        let de: ListPeersResponse = serde_json::from_str(&res).unwrap();
        Ok(de.result.peers)
    }


    pub async fn list_nodes(&self) -> Result<Vec<Node>, Error> {
        let req = Request::ListNodes(model::ListnodesRequest {id: None});
        let res = self.call(req).await?;
        log::trace!("{}", &res);
        let de: ListNodesResponse = serde_json::from_str(&res).unwrap();
        Ok(de.result.nodes)
    }

    pub async fn keysend_node(&self, pubkey: cln_rpc::primitives::PublicKey, amount: Amount) -> Result<(), Error> {
        log::info!("Keysending node {:?}, {:?}", pubkey, amount);
        let req = Request::KeySend(model::KeysendRequest { 
            destination: pubkey, 
            amount_msat: cln_rpc::primitives::Amount::from_msat(amount.msat()),
            label: None,
            maxfeepercent: None,
            retry_for: None,
            maxdelay: None,
            exemptfee: None,
            routehints: None,
            extratlvs: None,
        }
        );
        let res = self.call(req).await?;
        log::debug!("Keysend response {}", &res);
        let _de: KeysendResponseResponse = serde_json::from_str(&res).unwrap();
        
        Ok(())
    }
    
    // Randomize fee
    
    pub async fn randomize_fee(&self, short_channel_id: &String) -> Result<(), Error> {
        let random_ppm: u32 = random::<u32>() % 700 + 50;
        let random_base: u64 = random::<u64>() % 1500 + 1;
        let req = Request::SetChannel(model::SetchannelRequest {
            id: short_channel_id.to_string(),
            feeppm: Some(random_ppm),
            feebase: Some(cln_rpc::primitives::Amount::from_msat(random_base)),
            htlcmin: None,
            htlcmax: None,
            enforcedelay: None,
        });
        let res = self.call(req).await?;
        log::info!("Set channel: {:?}", res);
    
        Ok(())
    }
    
    // Randomly ping peer
    
    pub async fn random_ping_peer(&self, pubkey: cln_rpc::primitives::PublicKey) -> Result<(), Error> {
        
        let ping_len: u32 = random::<u32>();
        let pong_len: u32 = random::<u32>();
        let req = Request::Ping(model::PingRequest { 
            id: pubkey, 
            len: Some(ping_len.into()),
            pongbytes: Some(pong_len.into())
         });
         match self.call(req).await {
            Ok(res) => {
                log::info!("Pinged peer (Ping Length: {}, Pong Length: {}, Response: {})", ping_len, pong_len, res);
                Ok(())
            },
            Err(e) => Err(e)
         }    
    }

    pub async fn close_channel(&self, short_channel_id: &String) -> Result<String, Error> {
        let req = Request::Close(model::CloseRequest { 
            id: short_channel_id.to_string(),
            unilateraltimeout: None,
            destination: None,
            fee_negotiation_step: None,
            wrong_funding: None, 
            force_lease_closed: None,
            feerange: None,
        });
        self.call(req).await
    }

    
    
    

    pub async fn open_channel_to_node(&self, node: Node, size: u64) -> Result<String, Error> {
        let mut ipv4_address: Option<ListnodesNodesAddress>;
        ipv4_address = None;
        match node.addresses {
            Some(addresses) => {
                for address in addresses {
                    match address.item_type {
                        ListnodesNodesAddressType::IPV4 => { ipv4_address = Some(address) },
                        _ => { log::debug!("Not an IPV4")}
                    }
                }
            },
            None => {
                log::info!("Node does not have any addresses, bypassing");
                return Err(MyCustomError::NodeNotAddressableError.into())
            }
        }
        if ipv4_address.is_some() {
            let address = ipv4_address.unwrap();
            let req = Request::Connect(model::ConnectRequest { id: node.nodeid.to_string(), host: address.address, port: Some(address.port) });
            match self.call(req).await {
                Ok(res) => {
                    let _de: ConnectResponseResponse = serde_json::from_str(&res).unwrap();
                    log::info!("Peering success {:?}", res);
                },
                Err(_e) => {
                    return Err(MyCustomError::ConnectionFailedError.into())
                }
            }
        }
        let pubkey = node.nodeid;
        let amount = cln_rpc::primitives::AmountOrAll::Amount(cln_rpc::primitives::Amount::from_sat(size));
        let open_req = Request::FundChannel(model::FundchannelRequest {
            id: pubkey, 
            amount: amount,
            feerate: None,
            announce: None,
            minconf: None,
            push_msat: None,
            close_to: None,
            request_amt: None,
            compact_lease: None,
            utxos: None,
            mindepth: None,
            reserve: None,
        });
        log::info!("Opening channel (PeerID: {}, Size: {})", pubkey.to_string(), size);
        match self.call(open_req).await {
            Ok(res) => {
                log::info!("Opened channel: {:?}", res);
                let de: FundChannelResponseResponse = serde_json::from_str(&res).unwrap();
                return Ok(de.result.txid)
            },
            Err(e) => {
                log::error!("Unable to open channel: {:?}", e);
                return Err(e)
            }
        }
    
        
    }
}




// Config stuff

#[derive(Debug)]
pub struct Config {
    pub rpc_path: String,

    pub active: bool,
    pub open_probability: f64,
    pub close_probability: f64,

}

impl Default for Config {
    fn default() -> Self {
        Self { 
            active: true, 
            rpc_path: "lightning-rpc".to_string(),
            open_probability: 0.01,
            close_probability: 0.0005,
        }
    }
}

pub fn load_configuration(plugin: &Plugin<()>, config_holder: Arc<RwLock<Config>>) -> Result<(), Error> {
    let mut c = config_holder.write().unwrap();

    let active = match plugin.option("spaz-on-load") {
        Some(options::Value::Boolean(false)) => {
            log::debug!("`spaz-on-load` option is set to false.  Disabling");
            false
        }
        Some(options::Value::Boolean(true)) => {
            log::debug!("`spaz-on-load` option is set to true.  Enabling.");
            true
        }
        None => {
            log::info!("Missing 'spaz-on-load' option.  Disabling.");
            false
        }
        Some(o) => return Err(anyhow!("spaz-on-load is not a valid boolean: {:?}.", o)),
    };

    c.active = active;

    match plugin.option("spaz-rpc-path") {
        Some(options::Value::String(s)) => {
            c.rpc_path = s
        }
        None => {
            log::info!("Missing 'spaz-rpc-path' option.  Using default.");
        },
        Some(_) => {
            log::info!("Weird 'spaz-rpc-path' value.  Using default.");
        }
    };

    log::info!("Configuration loaded: {:?}", c);
    Ok(())
}

// CLN Stuff

// ListChannels
#[derive(Debug, Deserialize)]
pub struct ListFundsResponse {
    pub result: ListFundsResponseFunds,
}

#[derive(Debug, Deserialize)]
pub struct ListFundsResponseFunds {
    pub channels: Vec<Channel>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Channel {
    pub peer_id: String,
    pub connected: bool,
    pub our_amount_msat: Amount,
    pub amount_msat: Amount,
    pub funding_txid: String,
    pub funding_output: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_channel_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListPeersResponse {
    pub result: ListPeersResponsePeers,
}

#[derive(Debug, Deserialize)]
pub struct ListPeersResponsePeers {
    pub peers: Vec<Peer>,
}

#[derive(Debug, Deserialize)]
pub struct Peer {
    #[serde(alias = "id")]
    pub id: cln_rpc::primitives::PublicKey,
    #[serde(alias = "connected")]
    pub connected: bool,
}



// ListNodes

#[derive(Debug, Deserialize)]
pub struct ListNodesResponse {
    pub result: ListNodesResponseNodes,
}

#[derive(Debug, Deserialize)]
pub struct ListNodesResponseNodes {
    pub nodes: Vec<Node>,
}

#[derive(Debug, Deserialize)]
pub struct Node {
    #[serde(alias = "nodeid")]
    pub nodeid: cln_rpc::primitives::PublicKey,
    #[serde(alias = "last_timestamp", skip_serializing_if = "Option::is_none")]
    pub last_timestamp: Option<u32>,
    #[serde(alias = "alias", skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(alias = "color", skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(alias = "features", skip_serializing_if = "Option::is_none")]
    pub features: Option<String>,
    #[serde(alias = "addresses", skip_serializing_if = "crate::is_none_or_empty")]
    pub addresses: Option<Vec<ListnodesNodesAddress>>,
}

/// Type of connection
#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub enum ListnodesNodesAddressType {
    #[serde(rename = "dns")]
    DNS,
    #[serde(rename = "ipv4")]
    IPV4,
    #[serde(rename = "ipv6")]
    IPV6,
    #[serde(rename = "torv2")]
    TORV2,
    #[serde(rename = "torv3")]
    TORV3,
    #[serde(rename = "websocket")]
    WEBSOCKET,
}

impl TryFrom<i32> for ListnodesNodesAddressType {
    type Error = anyhow::Error;
    fn try_from(c: i32) -> Result<ListnodesNodesAddressType, anyhow::Error> {
        match c {
    0 => Ok(ListnodesNodesAddressType::DNS),
    1 => Ok(ListnodesNodesAddressType::IPV4),
    2 => Ok(ListnodesNodesAddressType::IPV6),
    3 => Ok(ListnodesNodesAddressType::TORV2),
    4 => Ok(ListnodesNodesAddressType::TORV3),
    5 => Ok(ListnodesNodesAddressType::WEBSOCKET),
            o => Err(anyhow::anyhow!("Unknown variant {} for enum ListnodesNodesAddressesType", o)),
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListnodesNodesAddress {
    // Path `ListNodes.nodes[].addresses[].type`
    #[serde(rename = "type")]
    pub item_type: ListnodesNodesAddressType,
    #[serde(alias = "port")]
    pub port: u16,
    #[serde(alias = "address", skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct KeysendResponseResponse { 
    pub result: model::KeysendResponse
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConnectResponseResponse { 
    pub result: model::ConnectResponse
}

#[derive(Clone, Debug, Deserialize)]
pub struct FundChannelResponseResponse {
    pub result: model::FundchannelResponse
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Amount {
    pub msat: u64,
}

impl Amount {
    pub fn from_msat(msat: u64) -> Amount {
        Amount { msat: msat }
    }
    pub fn from_sat(sat: u64) -> Amount {
        Amount { msat: 1_000 * sat }
    }
    pub fn from_btc(btc: u64) -> Amount {
        Amount {
            msat: 100_000_000_000 * btc,
        }
    }

    pub fn msat(&self) -> u64 {
        self.msat
    }
}

impl<'de> Deserialize<'de> for Amount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        let s: String = Deserialize::deserialize(deserializer)?;
        let ss: &str = &s;
        ss.try_into()
            .map_err(|_e| Error::custom("could not parse amount"))
    }
}

impl Serialize for Amount {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}msat", self.msat))
    }
}

impl TryFrom<&str> for Amount {
    type Error = Error;
    fn try_from(s: &str) -> Result<Amount> {
        let number: u64 = s
            .chars()
            .map(|c| c.to_digit(10))
            .take_while(|opt| opt.is_some())
            .fold(0, |acc, digit| acc * 10 + (digit.unwrap() as u64));

        let s = s.to_lowercase();
        if s.ends_with("msat") {
            Ok(Amount::from_msat(number))
        } else if s.ends_with("sat") {
            Ok(Amount::from_sat(number))
        } else if s.ends_with("btc") {
            Ok(Amount::from_btc(number))
        } else {
            Err(anyhow!("Unable to parse amount from string: {}", s))
        }
    }
}

impl From<Amount> for String {
    fn from(a: Amount) -> String {
        format!("{}msat", a.msat)
    }
}


use std::fmt;

#[derive(Debug)]
pub enum MyCustomError {
  NodeNotAddressableError,
  ConnectionFailedError,
}

impl std::error::Error for MyCustomError {}

impl fmt::Display for MyCustomError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      MyCustomError::NodeNotAddressableError => write!(f, "Node not addressable"),
      MyCustomError::ConnectionFailedError => write!(f, "Could not connect to node"),
    }
  }
}