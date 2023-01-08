#!/bin/bash
echo "Funding myself"
address=$(docker exec bitcoin bitcoin-cli -rpcwallet=rpcwallet --datadir=config getnewaddress)
my_addr=$(~/code/lightning/cli/lightning-cli --network=regtest newaddr bech32 | jq '.bech32' -r)
docker exec bitcoin bitcoin-cli -datadir=config -rpcwallet=rpcwallet sendtoaddress "$my_addr" 1
docker exec bitcoin bitcoin-cli -datadir=config -rpcwallet=rpcwallet sendtoaddress "$my_addr" 1
docker exec bitcoin bitcoin-cli -datadir=config -rpcwallet=rpcwallet sendtoaddress "$my_addr" 1
docker exec bitcoin bitcoin-cli -datadir=config -rpcwallet=rpcwallet sendtoaddress "$my_addr" 1
docker exec bitcoin bitcoin-cli -datadir=config -rpcwallet=rpcwallet sendtoaddress "$my_addr" 1
docker exec bitcoin bitcoin-cli -datadir=config -rpcwallet=rpcwallet sendtoaddress "$my_addr" 1
docker exec bitcoin bitcoin-cli --datadir=config generatetoaddress 6 $address

echo "Gathering pubkeys"
pubkey_r=$(docker exec cln-remote lightning-cli --network=regtest getinfo | jq '.id' -r)
pubkey_c1=$(docker exec cln-c1 lightning-cli --network=regtest getinfo | jq '.id' -r)
pubkey_c2=$(docker exec cln-c2 lightning-cli --network=regtest getinfo | jq '.id' -r)
pubkey_c3=$(docker exec cln-c3 lightning-cli --network=regtest getinfo | jq '.id' -r)

pubkey_lnd=$(docker exec lnd lncli --network=regtest getinfo | jq '.identity_pubkey' -r)
pubkey_lnd2=$(docker exec lnd2 lncli --network=regtest getinfo | jq '.identity_pubkey' -r)

port_r=50252 
port_c1=50257 
port_c2=50243 
port_c3=50255 
port_lnd=61814
port_lnd2=50254 

echo "Peering"
~/code/lightning/cli/lightning-cli --network=regtest connect $pubkey_r 127.0.0.1 $port_r
~/code/lightning/cli/lightning-cli --network=regtest connect $pubkey_c1 127.0.0.1 $port_c1
~/code/lightning/cli/lightning-cli --network=regtest connect $pubkey_c2 127.0.0.1 $port_c2
~/code/lightning/cli/lightning-cli --network=regtest connect $pubkey_c3 127.0.0.1 $port_c3
~/code/lightning/cli/lightning-cli --network=regtest connect $pubkey_lnd 127.0.0.1 $port_lnd
~/code/lightning/cli/lightning-cli --network=regtest connect $pubkey_lnd2 127.0.0.1 $port_lnd2

echo "Opening channels"
~/code/lightning/cli/lightning-cli --network=regtest fundchannel $pubkey_r 1000000
~/code/lightning/cli/lightning-cli --network=regtest fundchannel $pubkey_c1 1000000
~/code/lightning/cli/lightning-cli --network=regtest fundchannel $pubkey_c2 1000000
~/code/lightning/cli/lightning-cli --network=regtest fundchannel $pubkey_c3 1000000
~/code/lightning/cli/lightning-cli --network=regtest fundchannel $pubkey_lnd 1000000
~/code/lightning/cli/lightning-cli --network=regtest fundchannel $pubkey_lnd2 1000000


