use std::{io, net::SocketAddr, sync::Arc};

use local_ip_address::local_ip;
use quinn::{Endpoint, rustls::pki_types::CertificateDer};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::{
    StreamType,
    receivers::{ServerArgs, StreamTypeWithArgs, receivers::rtp_receiver},
};

/// inject an instance of a peer manager for the server to manage
pub async fn run_signaling_server(
    audio_endpoint: Endpoint,
    audio_cert: CertificateDer<'static>,
    video_endpoint: Endpoint,
    video_cert: CertificateDer<'static>,
    ssrc: u32,
) -> io::Result<()> {
    let audio_addr = audio_endpoint.local_addr()?;
    let video_addr = video_endpoint.local_addr()?;

    accept_endpoints(StreamType::Audio, audio_endpoint);
    accept_endpoints(StreamType::Video, video_endpoint);

    let local_ip = local_ip().unwrap();
    let listener = TcpListener::bind(local_ip.to_string() + ":8084")
        .await
        .unwrap();

    println!("Signalling running on: {}", listener.local_addr().unwrap());

    let audio_cert = Arc::new(audio_cert);
    let video_cert = Arc::new(video_cert);

    loop {
        let (mut socket, client_addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("Failed to accept connection: {}", e);
                continue;
            }
        };

        println!("Request from {}", client_addr.to_string());

        let audio_cert= audio_cert.clone();
        let video_cert = video_cert.clone();

        tokio::spawn(async move {
            let mut buffer = [0; 1500];

            let bytes_read = socket.read(&mut buffer).await?;
            if bytes_read == 0 {
                return Ok(());
            }

            // parsing the request
            let request: ServerArgs = serde_json::from_slice(&buffer[..bytes_read]).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Could not parse request. {}", e),
                )
            })?;

            let (request_socket, benchmark_type, cert) = match request.stream_type {
                StreamTypeWithArgs::Audio {sample_rate: _, channels: _,} => {
                    (audio_addr, StreamTypeWithArgs::BenchmarkAudio, audio_cert)
                },
                StreamTypeWithArgs::Video { pps: _, sps: _ } => {
                    (video_addr, StreamTypeWithArgs::BenchmarkVideo, video_cert)
                }
                _ => {
                    return Err(io::Error::new(
                        std::io::ErrorKind::NetworkUnreachable,
                        "Should not be receiving from another benchmarker",
                    ));
                }
            };

            if let Err(e) = handle_signaling_client(&mut socket, request_socket, ssrc, benchmark_type, cert.to_vec()).await
            {
                eprintln!("Signaling error with {}: {}", client_addr, e);
                return Err(e)
            }

            Ok(())
        });
    }
}

async fn handle_signaling_client(
    socket: &mut TcpStream,
    addr: SocketAddr,
    ssrc: u32,
    benchmark_type: StreamTypeWithArgs, 
    cert: Vec<u8>
) -> io::Result<()> {
    let response = ServerArgs {
        signaling_address: socket.local_addr().unwrap().to_string(),
        local_rtp_address: addr.to_string(),
        stream_type: benchmark_type,
        peer_signalling_addresses: Vec::new(),
        ssrc,
        cert
    };

    socket
        .write_all(&serde_json::to_string(&response)?.as_bytes())
        .await?;

    Ok(())
}

fn accept_endpoints(stream_type: StreamType, endpoint: Endpoint) {

    println!("available to accept endpoints {:?}", stream_type);
    tokio::spawn(async move {
        let local_addr = endpoint.local_addr().unwrap();

        while let Some(incoming) = endpoint.accept().await {
            println!("INCOMING ENDPOINT!");
            let connection = incoming.await.unwrap();
            tokio::spawn(async move {
                if let Err(e) = rtp_receiver(connection, stream_type, local_addr).await {
                    eprintln!("Error occured: {}", e)
                }
            });
        }
    });
}
