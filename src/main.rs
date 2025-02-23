mod network;

use network::discovery::start_network;
use network::peer::Peer; // Make sure to import the start_network function
#[tokio::main]
async fn main() {
    let my_peer = Peer::new();
    println!("Peer created: {:?}", my_peer.id);
    // Start the network discovery and peer-to-peer network
    if let Err(e) = start_network(&my_peer).await {
        eprintln!("Error starting network: {}", e);
    }
    // Sleep to keep process alive (for testing)
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
}
