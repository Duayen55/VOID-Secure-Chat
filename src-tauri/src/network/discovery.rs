use libp2p::kad::{Behaviour as Kademlia, Event as KademliaEvent, store::MemoryStore};
use libp2p::mdns::{tokio::Behaviour as Mdns, Event as MdnsEvent};
use libp2p::swarm::NetworkBehaviour;

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "DiscoveryEvent")]
pub struct DiscoveryBehaviour {
    pub kademlia: Kademlia<MemoryStore>,
    pub mdns: Mdns,
}

#[derive(Debug)]
pub enum DiscoveryEvent {
    Kademlia(KademliaEvent),
    Mdns(MdnsEvent),
}

impl From<KademliaEvent> for DiscoveryEvent {
    fn from(event: KademliaEvent) -> Self {
        DiscoveryEvent::Kademlia(event)
    }
}

impl From<MdnsEvent> for DiscoveryEvent {
    fn from(event: MdnsEvent) -> Self {
        DiscoveryEvent::Mdns(event)
    }
}
