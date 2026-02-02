
use clap::Parser;
use std::error::Error;
use std::path::PathBuf;
use crate::network::swarm::{build_swarm, VoidEvent, SignalingRequest, SignalingResponse};
use libp2p::{
    Multiaddr, futures::StreamExt, swarm::SwarmEvent,
    request_response::Message,
};
use tokio::io::{self, AsyncBufReadExt};
use crate::network::utils;
use rusqlite::{params, Connection};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    #[arg(long)]
    pub port: Option<u16>,

    #[arg(long)]
    pub db: Option<String>,
    
    #[arg(long)]
    pub dial: Option<String>,
}

pub async fn run_cli(port: u16, db_path: String, dial_addr: Option<String>) -> Result<(), Box<dyn Error>> {
    // Setup logging
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
    
    log::info!("Starting VOID CLI Mode");
    log::info!("Port: {}", port);
    log::info!("DB: {}", db_path);

    // Setup DB
    let conn = Connection::open(&db_path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY,
            peer_id TEXT NOT NULL,
            content TEXT NOT NULL,
            is_sent BOOLEAN NOT NULL,
            timestamp INTEGER NOT NULL
        )",
        [],
    )?;

    // Build Swarm
    let mut swarm = build_swarm().await.map_err(|e| format!("Failed to build swarm: {}", e))?;

    // Listen on TCP
    let listen_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", port).parse()?;
    swarm.listen_on(listen_addr.clone())?;
    log::info!("Listening on {}", listen_addr);

    // Hardcoded Public Relays (Bootnodes)
    let bootnodes = [
        "/dnsaddr/bootstrap.libp2p.io/p2p/QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN",
        "/dnsaddr/bootstrap.libp2p.io/p2p/QmQCU2EcMqAqQPR2i9bChDtGNJchTeq5s0GNHw5zXIov6U",
        "/dnsaddr/bootstrap.libp2p.io/p2p/QmbLHAnMoJPWSCR5Zhtx6BHJX9CkJv68846kJcCPaQFjNA",
        "/dnsaddr/bootstrap.libp2p.io/p2p/QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1Ubuu79rfVP3",
    ];

    for peer in bootnodes {
        if let Ok(addr) = peer.parse::<Multiaddr>() {
            log::info!("Dialing bootnode: {}", addr);
            if let Err(e) = swarm.dial(addr) {
                log::warn!("Failed to dial bootnode: {}", e);
            }
        }
    }

    // Listen on Relay (p2p-circuit)
    // We need to wait until we are connected to a relay to listen effectively, 
    // but we can try adding the listen addr now.
    let relay_addr: Multiaddr = "/p2p-circuit".parse()?;
    swarm.listen_on(relay_addr)?;
    log::info!("Attempting to listen on relay...");

    // Initial Dial if provided
    if let Some(code) = dial_addr {
        match utils::parse_void_code(&code) {
             Ok(addr) => {
                 log::info!("Dialing initial peer: {}", addr);
                 if let Err(e) = swarm.dial(addr) {
                     log::error!("Initial Dial Error: {}", e);
                 }
             }
             Err(e) => log::error!("Invalid initial dial code: {}", e),
        }
    }

    // Stdin reader
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    println!("Commands:");
    println!("  dial <void_code>  - Connect to a peer");
    println!("  send <peer_id> <msg> - Send message");
    println!("  info - Show my info");
    println!("  exit - Quit");

    loop {
        tokio::select! {
            line = stdin.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.is_empty() { continue; }
                        match parts[0] {
                            "dial" => {
                                if parts.len() < 2 {
                                    println!("Usage: dial <void_code>");
                                } else {
                                    match utils::parse_void_code(parts[1]) {
                                        Ok(addr) => {
                                            println!("Dialing {}", addr);
                                            if let Err(e) = swarm.dial(addr) {
                                                println!("Dial Error: {}", e);
                                            }
                                        }
                                        Err(e) => println!("Invalid code: {}", e),
                                    }
                                }
                            }
                            "send" => {
                                if parts.len() < 3 {
                                    println!("Usage: send <peer_id> <msg>");
                                } else {
                                    let peer_id_str = parts[1];
                                    let msg = parts[2..].join(" ");
                                    match peer_id_str.parse::<libp2p::PeerId>() {
                                        Ok(peer_id) => {
                                            println!("Sending to {}: {}", peer_id, msg);
                                            swarm.behaviour_mut().signaling.send_request(&peer_id, SignalingRequest(msg.clone()));
                                            
                                            // Log to DB
                                            conn.execute(
                                                "INSERT INTO messages (peer_id, content, is_sent, timestamp) VALUES (?1, ?2, ?3, ?4)",
                                                params![peer_id_str, msg, true, chrono::Utc::now().timestamp()],
                                            ).unwrap_or_else(|e| {
                                                println!("DB Error: {}", e);
                                                0
                                            });
                                        }
                                        Err(e) => println!("Invalid PeerId: {}", e),
                                    }
                                }
                            }
                            "info" => {
                                let local_peer_id = *swarm.local_peer_id();
                                println!("My PeerId: {}", local_peer_id);
                                println!("Listeners:");
                                let mut relay_found = false;
                                for addr in swarm.listeners() {
                                    println!(" - {}", addr);
                                    if addr.to_string().contains("p2p-circuit") {
                                        relay_found = true;
                                        let code = utils::generate_void_code(addr);
                                        println!("VOID CODE (Relay): {}", code);
                                    }
                                }
                                if !relay_found {
                                    println!("(Not listening on Relay yet. Wait for connection...)");
                                }
                            }
                            "exit" => break,
                            _ => println!("Unknown command"),
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        println!("Error reading stdin: {}", e);
                        break;
                    }
                }
            }
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        log::info!("New Listen Addr: {}", address);
                        if address.to_string().contains("p2p-circuit") {
                            let code = utils::generate_void_code(&address);
                            println!("\n*** READY ***");
                            println!("VOID CODE: {}", code);
                            println!("*************\n");
                        }
                    }
                    SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                        log::info!("Peer connected: {}", peer_id);
                        log::info!("Endpoint: {:?}", endpoint);
                        log::info!("Secure channel established");
                    }
                    SwarmEvent::Behaviour(VoidEvent::Signaling(event)) => {
                        match event {
                            libp2p::request_response::Event::Message { peer, message, .. } => {
                                match message {
                                    Message::Request { request, channel, .. } => {
                                        println!("\n[Message from {}]: {}", peer, request.0);
                                        // Log to DB
                                        conn.execute(
                                            "INSERT INTO messages (peer_id, content, is_sent, timestamp) VALUES (?1, ?2, ?3, ?4)",
                                            params![peer.to_string(), request.0, false, chrono::Utc::now().timestamp()],
                                        ).unwrap_or_else(|e| {
                                            println!("DB Error: {}", e);
                                            0
                                        });

                                        let _ = swarm.behaviour_mut().signaling.send_response(channel, SignalingResponse("ACK".into()));
                                    }
                                    Message::Response { .. } => {
                                        log::info!("Message delivered to {}", peer);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                     SwarmEvent::Behaviour(VoidEvent::RelayClient(e)) => {
                        log::debug!("Relay Event: {:?}", e);
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
