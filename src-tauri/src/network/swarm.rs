use super::discovery::{DiscoveryBehaviour, DiscoveryEvent};
use anyhow::Result;
use libp2p::{
    Multiaddr, PeerId, SwarmBuilder, autonat, dcutr, gossipsub, identify, noise, ping, relay,
    request_response::{self, ProtocolSupport},
    swarm::NetworkBehaviour,
    tcp, yamux,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalingRequest(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalingResponse(pub String);

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "VoidEvent")]
pub struct VoidBehaviour {
    pub discovery: DiscoveryBehaviour,
    pub gossipsub: gossipsub::Behaviour,
    pub autonat: autonat::Behaviour,
    pub relay_client: relay::client::Behaviour,
    pub dcutr: dcutr::Behaviour,
    pub identify: identify::Behaviour,
    pub ping: ping::Behaviour,
    pub signaling: request_response::cbor::Behaviour<SignalingRequest, SignalingResponse>,
}

#[derive(Debug)]
pub enum VoidEvent {
    Discovery(DiscoveryEvent),
    Gossipsub(gossipsub::Event),
    Autonat(autonat::Event),
    RelayClient(relay::client::Event),
    Dcutr(dcutr::Event),
    Identify(identify::Event),
    Ping(ping::Event),
    Signaling(request_response::Event<SignalingRequest, SignalingResponse>),
}

impl From<DiscoveryEvent> for VoidEvent {
    fn from(event: DiscoveryEvent) -> Self {
        VoidEvent::Discovery(event)
    }
}

impl From<gossipsub::Event> for VoidEvent {
    fn from(event: gossipsub::Event) -> Self {
        VoidEvent::Gossipsub(event)
    }
}

impl From<autonat::Event> for VoidEvent {
    fn from(event: autonat::Event) -> Self {
        VoidEvent::Autonat(event)
    }
}

impl From<relay::client::Event> for VoidEvent {
    fn from(event: relay::client::Event) -> Self {
        VoidEvent::RelayClient(event)
    }
}

impl From<dcutr::Event> for VoidEvent {
    fn from(event: dcutr::Event) -> Self {
        VoidEvent::Dcutr(event)
    }
}

impl From<identify::Event> for VoidEvent {
    fn from(event: identify::Event) -> Self {
        VoidEvent::Identify(event)
    }
}

impl From<ping::Event> for VoidEvent {
    fn from(event: ping::Event) -> Self {
        VoidEvent::Ping(event)
    }
}

impl From<request_response::Event<SignalingRequest, SignalingResponse>> for VoidEvent {
    fn from(event: request_response::Event<SignalingRequest, SignalingResponse>) -> Self {
        VoidEvent::Signaling(event)
    }
}

pub async fn build_swarm() -> Result<libp2p::Swarm<VoidBehaviour>> {
    let local_key = libp2p::identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());

    println!("Local PeerID: {}", local_peer_id);

    let mut swarm = SwarmBuilder::with_existing_identity(local_key.clone())
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_quic()
        .with_dns()?
        .with_relay_client(noise::Config::new, yamux::Config::default)?
        .with_behaviour(|key, relay_client| {
            // Gossipsub
            let message_id_fn = |message: &gossipsub::Message| {
                let mut s = std::collections::hash_map::DefaultHasher::new();
                use std::hash::{Hash, Hasher};
                message.data.hash(&mut s);
                gossipsub::MessageId::from(s.finish().to_string())
            };

            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(10))
                .validation_mode(gossipsub::ValidationMode::Strict)
                .message_id_fn(message_id_fn)
                .build()
                .map_err(|msg| std::io::Error::new(std::io::ErrorKind::Other, msg))?;

            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            )?;

            // Discovery (Kademlia + mDNS)
            let kademlia_store = libp2p::kad::store::MemoryStore::new(local_peer_id);
            let mut kademlia = libp2p::kad::Behaviour::new(local_peer_id, kademlia_store);

            // Bootnodes (IPFS/Libp2p)
            let bootnodes = [
                "/dnsaddr/bootstrap.libp2p.io/p2p/QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN",
                "/dnsaddr/bootstrap.libp2p.io/p2p/QmQCU2EcMqAqQPR2i9bChDtGNJchTeq5s0GNHw5zXIov6U",
                "/dnsaddr/bootstrap.libp2p.io/p2p/QmbLHAnMoJPWSCR5Zhtx6BHJX9CkJv68846kJcCPaQFjNA",
                "/dnsaddr/bootstrap.libp2p.io/p2p/QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1Ubuu79rfVP3",
            ];

            for peer in bootnodes {
                if let Ok(multiaddr) = Multiaddr::from_str(peer) {
                    if let Some(peer_id) = multiaddr.iter().find_map(|p| match p {
                        libp2p::multiaddr::Protocol::P2p(id) => Some(id),
                        _ => None,
                    }) {
                        kademlia.add_address(&peer_id, multiaddr);
                    }
                }
            }

            let mdns = libp2p::mdns::tokio::Behaviour::new(
                libp2p::mdns::Config::default(),
                local_peer_id,
            )?;

            let discovery = DiscoveryBehaviour { kademlia, mdns };

            // AutoNAT
            let autonat = autonat::Behaviour::new(
                local_peer_id,
                autonat::Config {
                    only_global_ips: true,
                    ..Default::default()
                },
            );

            // DCUTR (Direct Connection Upgrade Through Relay)
            let dcutr = dcutr::Behaviour::new(local_peer_id);

            // Identify
            let identify = identify::Behaviour::new(identify::Config::new(
                "void/1.0.0".to_string(),
                key.public(),
            ));

            // Ping
            let ping = ping::Behaviour::new(ping::Config::new());

            // Signaling (Request-Response)
            let signaling = request_response::cbor::Behaviour::new(
                [(
                    libp2p::StreamProtocol::new("/void/signaling/1.0.0"),
                    ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            );

            Ok(VoidBehaviour {
                discovery,
                gossipsub,
                autonat,
                relay_client,
                dcutr,
                identify,
                ping,
                signaling,
            })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    // Listen on all interfaces
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
    swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;

    Ok(swarm)
}
