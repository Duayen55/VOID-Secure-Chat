use anyhow::Result;
use libp2p::{
    PeerId, SwarmBuilder, identify, noise, ping, relay,
    request_response::{self, ProtocolSupport},
    tcp, yamux, websocket, dns,
    swarm::NetworkBehaviour,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalingRequest(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalingResponse(pub String);

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "VoidEvent")]
pub struct VoidBehaviour {
    pub relay_client: relay::client::Behaviour,
    pub identify: identify::Behaviour,
    pub ping: ping::Behaviour,
    pub signaling: request_response::cbor::Behaviour<SignalingRequest, SignalingResponse>,
}

#[derive(Debug)]
pub enum VoidEvent {
    RelayClient(relay::client::Event),
    Identify(identify::Event),
    Ping(ping::Event),
    Signaling(request_response::Event<SignalingRequest, SignalingResponse>),
}

impl From<relay::client::Event> for VoidEvent {
    fn from(event: relay::client::Event) -> Self {
        VoidEvent::RelayClient(event)
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
        .with_websocket(
            websocket::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .await?
        .with_dns()?
        .with_relay_client(noise::Config::new, yamux::Config::default)?
        .with_behaviour(|key, relay_client| {
            // Identify
            let identify = identify::Behaviour::new(identify::Config::new(
                "void/1.0.1".to_string(), // Updated version
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
                relay_client,
                identify,
                ping,
                signaling,
            })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    Ok(swarm)
}
