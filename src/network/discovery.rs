use crate::network::peer::Peer;
use libp2p::{
    kad::{self, store::MemoryStore},
    noise,
    swarm::NetworkBehaviour,
    tcp, yamux,
};
use std::error::Error;

#[derive(NetworkBehaviour)]
pub struct Behaviour {
    kademlia: kad::Behaviour<MemoryStore>,
}

pub async fn start_network(peer: &Peer) -> Result<(), Box<dyn Error>> {
    // let peer = Peer::new(); // Create a peer instance
    //let keypair = &peer.keypair;

    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(peer.keypair.clone())
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            move |keypair: &libp2p::identity::Keypair| noise::Config::new(keypair),
            || yamux::Config::default(),
        )?
        .with_behaviour(|_| {
            Ok(Behaviour {
                kademlia: kad::Behaviour::new(
                    peer.id,                   // Use peer's ID for the DHT
                    MemoryStore::new(peer.id), // Store DHT data in-memory
                ),
            })
        })?
        .build();

    println!("Peer {} started networking!", peer.id);

    Ok(())
}
