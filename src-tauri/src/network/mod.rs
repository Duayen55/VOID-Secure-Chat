pub mod discovery;
pub mod swarm;
pub mod utils;

use crate::network::swarm::{SignalingRequest, SignalingResponse, VoidEvent};
use libp2p::{Multiaddr, PeerId, futures::StreamExt, swarm::SwarmEvent};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{Mutex, mpsc, oneshot};

// Command enum to send instructions to the swarm task
#[derive(Debug)]
pub enum NetworkCommand {
    Dial(PeerId),
    DialAddress(Multiaddr),
    GetIdentity(oneshot::Sender<(PeerId, Vec<Multiaddr>)>),
    SendSignal(PeerId, String),
}

// State managed by Tauri
pub struct NetworkState {
    pub sender: Arc<Mutex<Option<mpsc::Sender<NetworkCommand>>>>,
}

impl NetworkState {
    pub fn new() -> Self {
        Self {
            sender: Arc::new(Mutex::new(None)),
        }
    }
}

#[tauri::command]
pub async fn send_signal(
    peer_id: String,
    payload: String,
    state: State<'_, NetworkState>,
) -> Result<(), String> {
    let peer_id = peer_id.parse::<PeerId>().map_err(|e| e.to_string())?;
    let sender_guard = state.sender.lock().await;

    if let Some(tx) = sender_guard.as_ref() {
        tx.send(NetworkCommand::SendSignal(peer_id, payload))
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Node not running".into())
    }
}

#[tauri::command]
pub async fn start_node(state: State<'_, NetworkState>, app: AppHandle) -> Result<String, String> {
    let mut sender_guard = state.sender.lock().await;
    if sender_guard.is_some() {
        return Ok("Node already running".into());
    }

    let (tx, mut rx) = mpsc::channel(32);
    *sender_guard = Some(tx);

    // Spawn the swarm task
    tokio::spawn(async move {
        match swarm::build_swarm().await {
            Ok(mut swarm) => {
                println!("Swarm initialized successfully");

                // Main Event Loop
                loop {
                    tokio::select! {
                        // Handle commands from Tauri
                        Some(cmd) = rx.recv() => {
                            match cmd {
                                NetworkCommand::Dial(peer_id) => {
                                    println!("Dialing peer: {}", peer_id);
                                    if let Err(e) = swarm.dial(peer_id) {
                                        println!("Failed to dial: {}", e);
                                    }
                                }
                                NetworkCommand::DialAddress(addr) => {
                                    println!("Dialing address: {}", addr);
                                    if let Err(e) = swarm.dial(addr) {
                                        println!("Failed to dial: {}", e);
                                    }
                                }
                                NetworkCommand::GetIdentity(reply_tx) => {
                                    let peer_id = *swarm.local_peer_id();
                                    // Collect external addresses first, then listeners
                                    let mut addrs: Vec<Multiaddr> = swarm.external_addresses().map(|a| a.clone()).collect();
                                    if addrs.is_empty() {
                                        addrs = swarm.listeners().map(|a| a.clone()).collect();
                                    }
                                    let _ = reply_tx.send((peer_id, addrs));
                                }
                                NetworkCommand::SendSignal(peer_id, payload) => {
                                    swarm.behaviour_mut().signaling.send_request(&peer_id, SignalingRequest(payload));
                                }
                            }
                        }

                        // Handle Swarm Events
                        event = swarm.select_next_some() => {
                            match event {
                                SwarmEvent::NewListenAddr { address, .. } => {
                                    println!("Listening on {:?}", address);
                                    let _ = app.emit("network-event", format!("Listening on {:?}", address));
                                }
                                SwarmEvent::Behaviour(VoidEvent::Discovery(discovery::DiscoveryEvent::Mdns(event))) => {
                                    println!("MDNS Event: {:?}", event);
                                }
                                SwarmEvent::Behaviour(VoidEvent::Discovery(discovery::DiscoveryEvent::Kademlia(_event))) => {
                                    // Reduce log spam for Kademlia
                                    // println!("Kademlia Event: {:?}", event);
                                }
                                SwarmEvent::Behaviour(VoidEvent::Autonat(event)) => {
                                    println!("AutoNAT Event: {:?}", event);
                                    // If we confirmed our public address, emit it
                                    if let libp2p::autonat::Event::StatusChanged { new, .. } = event {
                                        if let libp2p::autonat::NatStatus::Public(addr) = new {
                                            let _ = app.emit("network-event", format!("Public Address Confirmed: {:?}", addr));
                                        }
                                    }
                                }
                                SwarmEvent::Behaviour(VoidEvent::Signaling(event)) => {
                                    match event {
                                        libp2p::request_response::Event::Message { peer, message, .. } => {
                                            match message {
                                                libp2p::request_response::Message::Request { request, channel, .. } => {
                                                    println!("Received Signal from {}: {}", peer, request.0);
                                                    let _ = app.emit("signal-event", serde_json::json!({
                                                        "peerId": peer.to_string(),
                                                        "payload": request.0
                                                    }));
                                                    let _ = swarm.behaviour_mut().signaling.send_response(channel, SignalingResponse("ACK".into()));
                                                }
                                                _ => {}
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to build swarm: {}", e);
            }
        }
    });

    Ok("Node started".into())
}

#[tauri::command]
pub async fn dial_peer(peer_id: String, state: State<'_, NetworkState>) -> Result<(), String> {
    let peer_id = peer_id.parse::<PeerId>().map_err(|e| e.to_string())?;
    let sender_guard = state.sender.lock().await;

    if let Some(tx) = sender_guard.as_ref() {
        tx.send(NetworkCommand::Dial(peer_id))
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Node not running".into())
    }
}

#[tauri::command]
pub async fn connect_via_code(code: String, state: State<'_, NetworkState>) -> Result<(), String> {
    let multiaddr = utils::parse_void_code(&code)?;
    let sender_guard = state.sender.lock().await;

    if let Some(tx) = sender_guard.as_ref() {
        tx.send(NetworkCommand::DialAddress(multiaddr))
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Node not running".into())
    }
}

#[tauri::command]
pub async fn get_my_void_code(state: State<'_, NetworkState>) -> Result<String, String> {
    let (tx, rx) = oneshot::channel();
    let sender_guard = state.sender.lock().await;

    if let Some(sender) = sender_guard.as_ref() {
        sender
            .send(NetworkCommand::GetIdentity(tx))
            .await
            .map_err(|e| e.to_string())?;

        let (peer_id, addrs) = rx.await.map_err(|e| e.to_string())?;

        // Find a suitable IP and Port
        // Logic: Look for non-local IP if possible.
        // If listeners are 0.0.0.0, we can't really use that for the code unless we resolve local IP.
        // But for now, let's try to find a valid IP.

        let mut best_ip = "127.0.0.1".to_string();
        let mut best_port = 0;

        for addr in addrs {
            let mut ip = None;
            let mut port = None;

            for protocol in addr.iter() {
                match protocol {
                    libp2p::multiaddr::Protocol::Ip4(i) => {
                        if !i.is_loopback() && !i.is_unspecified() {
                            ip = Some(i.to_string());
                        } else if i.is_loopback() {
                            // keep loopback if nothing else
                        }
                    }
                    libp2p::multiaddr::Protocol::Udp(p) => port = Some(p),
                    libp2p::multiaddr::Protocol::Tcp(p) => port = Some(p),
                    _ => {}
                }
            }

            if let (Some(i), Some(p)) = (ip, port) {
                best_ip = i;
                best_port = p;
                // If it's not local, break, we found a good one
                if best_ip != "127.0.0.1" {
                    break;
                }
            }
        }

        Ok(utils::generate_void_code(peer_id, &best_ip, best_port))
    } else {
        Err("Node not running".into())
    }
}
