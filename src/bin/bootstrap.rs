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
    let keypair = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(keypair.public());
    println!("Bootstrap Node peer id: {:?}", peer_id);

    std::fs::write("bootstrap_peer_id", peer_id.to_string())
        .expect("Failed to write bootstrap peer ID to file");

    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            move |keypair: &libp2p::identity::Keypair| noise::Config::new(keypair),
            yamux::Config::default,
        )?
        .with_behaviour(|_| {
            let mut kad = kad::Behaviour::new(peer_id, MemoryStore::new(peer_id));
            kad.set_mode(Some(kad::Mode::Server));
            Ok(Behaviour {
                kademlia: kad,
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

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("Bootstrap node listening on: {:?}", address);
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                println!("Connection established with peer: {:?}", peer_id);
                // Add the peer to the DHT
                swarm
                    .behaviour_mut()
                    .kademlia
                    .add_address(&peer_id, listen_addr.clone());
                // Start a FindNode query for the new peer
                swarm.behaviour_mut().kademlia.get_closest_peers(peer_id);

                let key = kad::RecordKey::new(&peer_id.to_bytes());

                if let Err(e) = swarm.behaviour_mut().kademlia.start_providing(key) {
                    println!("Failed to advertise peer: {:?}", e);
                }
            }
            SwarmEvent::Behaviour(BehaviourEvent::Kademlia(kad::Event::RoutingUpdated {
                peer,
                ..
            })) => {
                println!("Routing table updated for peer: {:?}", peer);
            }
            SwarmEvent::Behaviour(event) => println!("Event received: {:?}", event),
            _ => {}
        }
    }
}
