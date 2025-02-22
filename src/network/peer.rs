use libp2p::identity;
use libp2p::PeerId;
use std::error::Error;

pub struct Peer {
    pub id: PeerId,
    pub keypair: identity::Keypair,
}
impl Peer {
    pub fn new() -> Self {
        let keypair = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());

        println!("Peer Id: {}", peer_id);

        Self {
            id: peer_id,
            keypair,
        }
    }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let my_peer = Peer::new();
    println!("succesfully created peer: {}", my_peer.id);

    Ok(())
}
