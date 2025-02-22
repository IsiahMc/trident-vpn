mod network;

use network::peer::Peer;

#[tokio::main]
async fn main() {
    let my_peer = Peer::new();
    println!("Peer created: {:?}", my_peer.id);

    // Sleep to keep process alive (for testing)
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
}
