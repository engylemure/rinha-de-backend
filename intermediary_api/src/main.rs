use rinha_grpc_server::server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    server().await
}
