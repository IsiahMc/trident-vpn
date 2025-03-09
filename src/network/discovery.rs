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
                println!("Connection established with bootstrap node: {:?}", peer_id);

                // Start a bootstrap process
                if let Err(e) = swarm.behaviour_mut().kademlia.bootstrap() {
                    println!("Failed to start bootstrap process: {:?}", e);
                } else {
                    println!("Started bootstrap process");
                }

                // Also start an explicit find_node query
                swarm.behaviour_mut().kademlia.get_closest_peers(peer_id);
                break;
            }
            SwarmEvent::ConnectionClosed { .. } => {
                println!("Connection to bootstrap node closed");
                break;
            }
            SwarmEvent::Behaviour(event) => println!("Event received: {:?}", event),
            _ => {}
        }
    }
    // Initiate bootstrap DHT query after adding address
    println!("Initiating bootstrap DHT query...");
    if let Err(err) = swarm.behaviour_mut().kademlia.bootstrap() {
        println!("Failed to start bootstrap process: {:?}", err);
    } else {
        println!("Bootstrap process started.");
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
            let mut kad = kad::Behaviour::new(peer.id, MemoryStore::new(peer.id));
            kad.set_mode(Some(Mode::Client));
            Ok(Behaviour {
                kademlia: kad,
                ping: ping::Behaviour::default(),
            })
        })?
        .with_swarm_config(|cfg: Config| {
            cfg.with_idle_connection_timeout(Duration::from_secs(u64::MAX))
        })
        .build();

    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
    println!("Peer {} started networking!", peer.id);

    let bootstrap_peer_id = std::fs::read_to_string("bootstrap_peer_id")
        .expect("Failed to read bootstrap peer ID from file")
        .parse()
        .expect("Failed to parse bootstrap peer ID");

    let bootstrap_addr: Multiaddr = "/ip4/127.0.0.1/tcp/50000".parse().unwrap();
    connect_to_bootstrap(&mut swarm, bootstrap_peer_id, bootstrap_addr).await;
    // Periodically try to discover peers
    let mut interval = tokio::time::interval(Duration::from_secs(30));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                 swarm.behaviour_mut().kademlia.get_closest_peers(peer.id);
                }
                event = swarm.select_next_some() => {
                    match event {
                        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        println!("Connected to peer: {:?}", peer_id);
                        // Try to discover more peers through this new peer
                        swarm.behaviour_mut().kademlia.get_closest_peers(peer_id);
                        }
                        SwarmEvent::Behaviour(BehaviourEvent::Kademlia(kad::Event::RoutingUpdated {
                            peer,
                            ..
                        })) => {
                            println!("Routing table updated for peer: {:?}", peer);
                        }
                        SwarmEvent::Behaviour(BehaviourEvent::Kademlia(
                            kad::Event::OutboundQueryProgressed { result, .. },
                        )) => match result {
                            QueryResult::GetClosestPeers(Ok(ok)) => {
                                println!("Found closest peers: {:?}", ok.peers);
                            }
                            QueryResult::Bootstrap(Ok(ok)) => {
                                println!(
                                    "Bootstrap progress: peer={:?}, remaining={}",
                                    ok.peer, ok.num_remaining
                                );
                            }
                            _ => {}
                        },
                        SwarmEvent::Behaviour(event) => println!("Event received: {:?}", event),
                        _ => {}
                    }
                    }

        }
    }
}
