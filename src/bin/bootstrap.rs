// bootstrap node so peers can know of other peers on startup
use libp2p::futures::StreamExt;
use libp2p::swarm::Config;
use libp2p::{identity, PeerId};
use libp2p::{
    kad::{self, store::MemoryStore},
    noise, ping,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr,
};
use std::error::Error;
use std::time::Duration;
use tokio::sync::mpsc;
#[derive(NetworkBehaviour)]
struct Behaviour {
    kademlia: kad::Behaviour<MemoryStore>,
    ping: ping::Behaviour,
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    println!("Starting program...");
    // set keypair and value
    let keypair = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(keypair.public());
    println!("Bootstrap Node peer id: {:?}", peer_id);

    // set Swarm
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            move |keypair: &libp2p::identity::Keypair| noise::Config::new(keypair),
            yamux::Config::default,
        )?
        .with_behaviour(|_| {
            Ok(Behaviour {
                kademlia: kad::Behaviour::new(
                    peer_id,                   // Use peer's ID for the DHT
                    MemoryStore::new(peer_id), // Store DHT data in-memory
                ),
                ping: ping::Behaviour::default(),
            })
        })?
        .with_swarm_config(|cfg: Config| {
            cfg.with_idle_connection_timeout(Duration::from_secs(u64::MAX))
        })
        .build();
    let listen_addr: Multiaddr = "/ip4/0.0.0.0/tcp/50000".parse()?;
    swarm.listen_on(listen_addr.clone())?;

    println!("Bootstrap node running at: {:?}", listen_addr);

    let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

    loop {
        tokio::select! {
            // Handle shutdown signal
            _ = shutdown_rx.recv() => {
                println!("Shutting down bootstrap node...");
                break;
            }
            // Handle events emitted by the Swarm
            event = swarm.select_next_some() => {
                match event {
                    // Print a message when a peer connects
                     SwarmEvent::Behaviour(BehaviourEvent::Kademlia(kad::Event::RoutingUpdated { peer, .. })) => {
                        println!("Peer connected: {:?}", peer);
                    }
                    // Handle when the node starts listening on an address
                    SwarmEvent::NewListenAddr { address, .. } => {
                        println!("Listening on: {:?}", address);
                    }
                    SwarmEvent::Behaviour(event) => println!("Event received from peer is {:?}", event),

                    // Catch-all pattern to handle any other events
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
