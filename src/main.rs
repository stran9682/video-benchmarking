use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
};

use local_ip_address::local_ip;
use video_server::quic::make_server_endpoint;
use video_server::receivers::signalling::run_signaling_server;

#[tokio::main]
async fn main() {
    if let Err(e) = network_loop().await {
        eprintln!("{}", e)
    }
}

async fn network_loop() -> io::Result<()> {
    println!("Starting up a listener for benchmarking");
    let local_ip = local_ip().unwrap();
    let addr = Ipv4Addr::from_str(&local_ip.to_string()).unwrap();

    let video_socket = SocketAddr::new(IpAddr::V4(addr), 8080);
    let audio_socket = SocketAddr::new(IpAddr::V4(addr), 8082);

    let (video_endpoint, video_server_cert) = make_server_endpoint(video_socket, &0)?;
    let (audio_endpoint, audio_server_cert) = make_server_endpoint(audio_socket, &0)?;

    run_signaling_server(
        audio_endpoint, 
        audio_server_cert, 
        video_endpoint, 
        video_server_cert, 
        0
    )
        .await
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::ConnectionAborted,
                format!("Signalling failure: {}", e),
            )
        })?;

    Ok(())
}
