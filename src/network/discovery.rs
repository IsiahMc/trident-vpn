use crate::network::peer::Peer;
use libp2p::futures::StreamExt;
use libp2p::swarm::Config;
use libp2p::{
    kad::{self, store::MemoryStore, Mode, QueryResult},
    noise, ping,
    swarm::{ConnectionHandler, NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId,
};
use std::error::Error;
use std::time::Duration;
use std::u64;
#[derive(NetworkBehaviour)]
pub struct Behaviour {
    kademlia: kad::Behaviour<MemoryStore>,
    ping: ping::Behaviour,
}
// connect to bootstrap node
pub async fn connect_to_bootstrap(
    swarm: &mut libp2p::Swarm<Behaviour>,
    bootstrap_peer_id: PeerId,
    bootstrap_addr: Multiaddr,
) {
    swarm
        .behaviour_mut()
        .kademlia
        .add_address(&bootstrap_peer_id, bootstrap_addr.clone());

    // Initiate bootstrap DHT query after adding address
    if let Err(err) = swarm.behaviour_mut().kademlia.bootstrap() {
        println!("Failed to start bootstrap process: {:?}", err);
    } else {
        println!("Bootstrap process started.");
    }
    // dial the bootstrap node
    match swarm.dial(bootstrap_addr.clone()) {
        Ok(_) => println!("Dialing bootstrap node at {}", bootstrap_addr),
        Err(err) => {
            println!("Failed to dial bootstrap node: {:?}", err);
            return;
        }
    }

    // Wait for connection confirmation
    loop {
        match swarm.select_next_some().await {
            SwarmEvent::ConnectionEstablished {
                peer_id,
                connection_id,
                ..
            } => {
                println!(
                    "Connection established with peer: {:?} (Connection ID: {:?})",
                    peer_id, connection_id
                );
                break; // Break once any connection is established
            }
            SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                println!(
                    "Connection closed with peer: {:?} (Cause: {:?})",
                    peer_id, cause
                );
                break; // Break once the connection is closed
            }
            SwarmEvent::Behaviour(event) => println!("Event received from peer is {:?}", event),
            _ => {
                // Log unexpected events
                // println!("Received an unexpected event: {:?}", event);
            }
        }
    }
}

pub async fn start_network(peer: &Peer) -> Result<(), Box<dyn Error>> {
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(peer.keypair.clone())
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            move |keypair: &libp2p::identity::Keypair| noise::Config::new(keypair),
            yamux::Config::default,
        )?
        .with_behaviour(|_| {
            Ok(Behaviour {
                kademlia: kad::Behaviour::new(
                    peer.id,                   // Use peer's ID for the DHT
                    MemoryStore::new(peer.id), // Store DHT data in-memory
                ),
                ping: ping::Behaviour::default(),
            })
        })?
        .with_swarm_config(|cfg: Config| {
            cfg.with_idle_connection_timeout(Duration::from_secs(u64::MAX))
        })
        .build();

    swarm.behaviour_mut().kademlia.set_mode(Some(Mode::Server));
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
    println!("Peer {} started networking!", peer.id);

    let bootstrap_addr: Multiaddr = "/ip4/127.0.0.1/tcp/50000".parse().unwrap();
    let bootstrap_peer_id: PeerId = PeerId::from_public_key(&peer.keypair.public());
    connect_to_bootstrap(&mut swarm, bootstrap_peer_id, bootstrap_addr).await;

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::Behaviour(BehaviourEvent::Kademlia(kad::Event::RoutingUpdated {
                peer,
                ..
            })) => {
                println!("Connected to peer: {:?}", peer);
            }
            SwarmEvent::Behaviour(BehaviourEvent::Kademlia(
                kad::Event::OutboundQueryProgressed { result, .. },
            )) => {
                if let QueryResult::GetClosestPeers(Ok(found)) = result {
                    println!("discovered peers: {:?}", found.peers);
                }
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("listening on: {:?}", address);
            }
            SwarmEvent::Behaviour(event) => println!("{event:?}"),
            _ => {}
        }
    }

    // Ok(())
}
