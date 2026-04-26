use std::{io, net::SocketAddr};

use local_ip_address::local_ip;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::receivers::{ServerArgs, StreamTypeWithArgs};

/// inject an instance of a peer manager for the server to manage
pub async fn run_signaling_server(
    audio_addr: SocketAddr,
    video_addr: SocketAddr,
    ssrc: u32,
) -> io::Result<()> {
    let local_ip = local_ip().unwrap();
    let listener = TcpListener::bind(local_ip.to_string() + ":8084")
        .await
        .unwrap();

    println!("Signalling running on: {}", listener.local_addr().unwrap());

    loop {
        let (mut socket, client_addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("Failed to accept connection: {}", e);
                continue;
            }
        };

        println!("Request from {}", client_addr.to_string());

        let audio_addr = audio_addr.clone();
        let video_addr = video_addr.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_signaling_client(&mut socket, audio_addr, video_addr, ssrc).await
            {
                eprintln!("Signaling error with {}: {}", client_addr, e);
            }
        });
    }
}

async fn handle_signaling_client(
    socket: &mut TcpStream,
    audio_addr: SocketAddr,
    video_addr: SocketAddr,
    ssrc: u32,
) -> io::Result<()> {
    let mut len_buf =[0u8; 4];
    socket.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    println!("packet size: {}", len);

    let mut buffer = vec![0u8; len];
    socket.read_exact(&mut buffer).await?;
    

    // parsing the request
    let request: ServerArgs = serde_json::from_slice(&buffer[..]).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Could not parse request. {}", e),
        )
    })?;

    let request_socket= match request.stream_type {
        StreamTypeWithArgs::Audio {sample_rate: _, channels: _,} => {
            audio_addr
        },
        StreamTypeWithArgs::Video { pps: _, sps: _ } => {
            video_addr
        }
    };

    let response = ServerArgs {
        signaling_address: socket.local_addr().unwrap().to_string(),
        local_rtp_address: request_socket.to_string(),
        stream_type: request.stream_type,
        peer_signalling_addresses: Vec::new(),
        ssrc,
    };

    socket
        .write_all(&serde_json::to_string(&response)?.as_bytes())
        .await?;

    Ok(())
}
