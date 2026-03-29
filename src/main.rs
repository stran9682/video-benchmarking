use std::{io};

use local_ip_address::local_ip;
use tokio::net::UdpSocket;
use video_server::{ receivers::{receivers::{rtp_audio_receiver, rtp_frame_receiver}, signalling::run_signaling_server}};

#[tokio::main]
async fn main() {
    tokio::spawn(async move {
        if let Err(e) = network_loop().await {
            eprintln!("{}", e)
        }
    });
}

async fn network_loop() -> io::Result<()> {
    println!("Starting up a listener for benchmarking");
    let local_ip = local_ip().unwrap();

    let video_socket = UdpSocket::bind(local_ip.to_string() + ":0").await?;
    let audio_socket = UdpSocket::bind(local_ip.to_string() + ":0").await?;

    let audio_addr = audio_socket.local_addr()?;
    let video_addr = video_socket.local_addr()?;
    tokio::spawn(async move {
        run_signaling_server(audio_addr, video_addr, 0).await
    });

    tokio::spawn(async move {
        rtp_audio_receiver(audio_socket, 48_000).await
    });

    tokio::spawn(async move {
        rtp_frame_receiver(video_socket, 90_000).await
    });

    Ok(())
}
