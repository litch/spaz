#[macro_use]
extern crate serde_json;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use anyhow::{anyhow, Error, Result};
use std::path::Path;
extern crate rand;

use cln_plugin::{Plugin};

use cln_rpc::{model, ClnRpc, Request};

use std::sync::{Arc, RwLock};

pub async fn disconnect_peer(pubkey: cln_rpc::primitives::Pubkey) -> Result<(), Error> {
    log::info!("Disconnecting from peer: {:?}", pubkey);
    let req = Request::Disconnect(model::DisconnectRequest { id: pubkey, force: Some(true) });
    let res = call(req).await?;
    Ok(())
}


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
    pub id: cln_rpc::primitives::Pubkey,
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

async fn call(request: Request) -> Result<String, Error> {
    let path = Path::new("lightning-rpc");
    let mut rpc = ClnRpc::new(path).await?;
    let response = rpc
        .call(request.clone())
        .await
        .map_err(|e| anyhow!("Error calling {:?}: {:?}", request, e))?;

    Ok(serde_json::to_string_pretty(&response)?)
}


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

    let active = false;

    Config {
        active
    }
    .make_current();
    log::info!("Configuration loaded: {:?}", Config::current());
    Ok(Config::current())
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