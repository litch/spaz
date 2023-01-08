#[macro_use]
extern crate serde_json;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use anyhow::{anyhow, Error, Result};
use std::path::Path;
extern crate rand;
use rand::{random};


use cln_plugin::{Plugin};

use cln_rpc::{model::{self, ConnectResponse}, ClnRpc, Request};

use std::sync::{Arc, RwLock};

pub async fn disconnect_peer(pubkey: cln_rpc::primitives::PublicKey) -> Result<(), Error> {
    log::info!("Disconnecting from peer: {:?}", pubkey);
    let req = Request::Disconnect(model::DisconnectRequest { id: pubkey, force: Some(true) });
    let res = call(req).await?;
    Ok(())
}
// Config stuff


pub async fn start_handler(
    _p: Plugin<()>, _v:serde_json::Value
) -> Result<serde_json::Value, Error> {
    log::info!("Plugin start requested");
    let c = Config::current();
    let active = true;
    Config {
        active
    }.make_current();

    Ok(json!("ok"))
}

pub async fn stop_handler(
    _p: Plugin<()>, _v:serde_json::Value
) -> Result<serde_json::Value, Error> {
    log::info!("Plugin stop requested");
    let c = Config::current();
    let active = false;
    Config {
        active
    }.make_current();

    Ok(json!("ok"))
}

#[derive(Default, Debug)]
pub struct Config {
    pub active: bool,
}

impl Config {
    pub fn default() -> Config {
        Config {
            active: true,
        }
    }

    pub fn current() -> Arc<Config> {
        CURRENT_CONFIG.with(|c| c.read().unwrap().clone())
    }
    pub fn make_current(self) {
        CURRENT_CONFIG.with(|c| *c.write().unwrap() = Arc::new(self))
    }
}

thread_local! {
    static CURRENT_CONFIG: RwLock<Arc<Config>> = RwLock::new(Default::default());
}

pub fn load_configuration(plugin: &Plugin<()>) -> Result<Arc<Config>, Error> {
    let c = Config::default();

    let active = true;

    Config {
        active
    }
    .make_current();
    log::info!("Configuration loaded: {:?}", Config::current());
    Ok(Config::current())
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
    pub state: ChannelState,
    pub our_amount_msat: Amount,
    pub amount_msat: Amount,
    pub funding_txid: String,
    pub funding_output: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_channel_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub enum ChannelState {
    OPENINGD,
    CHANNELD_AWAITING_LOCKIN,
    CHANNELD_NORMAL,
    CHANNELD_SHUTTING_DOWN,
    CLOSINGD_SIGEXCHANGE,
    CLOSINGD_COMPLETE,
    AWAITING_UNILATERAL,
    FUNDING_SPEND_SEEN,
    ONCHAIN,
    DUALOPENED_OPEN_INIT,
    DUALOPEND_AWAITING_LOCKIN,
}


pub async fn list_channels() -> Result<Vec<Channel>, Error> {
    let req = Request::ListFunds(model::ListfundsRequest { spent: Some(false)} );
    let res = call(req).await?;
    log::trace!("{}", &res);

    let de: ListFundsResponse = serde_json::from_str(&res).unwrap();
    Ok(de.result.channels)
} 

// ListPeers


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

pub async fn list_peers() -> Result<Vec<Peer>, Error> {
    let req = Request::ListPeers(model::ListpeersRequest { id: None, level: None });
    let res = call(req).await?;
    log::trace!("{}", &res);
    let de: ListPeersResponse = serde_json::from_str(&res).unwrap();
    Ok(de.result.peers)
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
    pub addresses: Option<Vec<ListnodesNodesAddresses>>,
}

/// Type of connection
#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub enum ListnodesNodesAddressesType {
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

impl TryFrom<i32> for ListnodesNodesAddressesType {
    type Error = anyhow::Error;
    fn try_from(c: i32) -> Result<ListnodesNodesAddressesType, anyhow::Error> {
        match c {
    0 => Ok(ListnodesNodesAddressesType::DNS),
    1 => Ok(ListnodesNodesAddressesType::IPV4),
    2 => Ok(ListnodesNodesAddressesType::IPV6),
    3 => Ok(ListnodesNodesAddressesType::TORV2),
    4 => Ok(ListnodesNodesAddressesType::TORV3),
    5 => Ok(ListnodesNodesAddressesType::WEBSOCKET),
            o => Err(anyhow::anyhow!("Unknown variant {} for enum ListnodesNodesAddressesType", o)),
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListnodesNodesAddresses {
    // Path `ListNodes.nodes[].addresses[].type`
    #[serde(rename = "type")]
    pub item_type: ListnodesNodesAddressesType,
    #[serde(alias = "port")]
    pub port: u16,
    #[serde(alias = "address", skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
}


pub async fn list_nodes() -> Result<Vec<Node>, Error> {
    let req = Request::ListNodes(model::ListnodesRequest {id: None});
    let res = call(req).await?;
    log::trace!("{}", &res);
    let de: ListNodesResponse = serde_json::from_str(&res).unwrap();
    Ok(de.result.nodes)
}

// Keysend a node

#[derive(Clone, Debug, Deserialize)]
pub struct KeysendResponseResponse { 
    pub result: model::KeysendResponse
}

pub async fn keysend_node(pubkey: cln_rpc::primitives::PublicKey, amount: Amount) -> Result<(), Error> {
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
    let res = call(req).await?;
    log::debug!("Keysend response {}", &res);
    let de: KeysendResponseResponse = serde_json::from_str(&res).unwrap();
    
    Ok(())
}

// Randomize fee

pub async fn randomize_fee(short_channel_id: &String) -> Result<(), Error> {
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
    let res = call(req).await?;
    log::info!("Set channel: {:?}", res);

    Ok(())
}

// Open channel

pub async fn open_channel(pubkey: cln_rpc::primitives::PublicKey, alias: String, size: Amount) -> Result<String, Error> {
    let req = Request::Connect(model::ConnectRequest { id: pubkey.to_string(), host: Some(alias), port: Some(9735) });
    let res = call(req).await?;
    log::info!("Tried peering! {:?}", res);
    
    let de: ConnectResponse = serde_json::from_str(&res).unwrap();

    Ok("Opened?".to_string())
} 

// General

async fn call(request: Request) -> Result<String, Error> {
    let path = Path::new("lightning-rpc");
    let mut rpc = ClnRpc::new(path).await?;
    let response = rpc
        .call(request.clone())
        .await
        .map_err(|e| anyhow!("Error calling {:?}: {:?}", request, e))?;

    Ok(serde_json::to_string_pretty(&response)?)
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