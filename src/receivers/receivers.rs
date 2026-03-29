use std::{io, time::{Duration, SystemTime, UNIX_EPOCH}};

use bytes::{BufMut, BytesMut};
use tokio::net::UdpSocket;

use crate::rtp::rtp_header::RTPHeader;

pub async fn rtp_audio_receiver(
    socket: UdpSocket,
    media_clock_rate: u32,
) -> io::Result<()> {
    let mut buffer = [0u8; 1500];

    loop {
        let (bytes_read, _) = socket.recv_from(&mut buffer).await?;

        //println!("Got a packet!");

        let now = SystemTime::now();

        let arrival_time = now.duration_since(UNIX_EPOCH);

        let Ok(arrival_time) = arrival_time else {  
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Time error, packet arrived earlier than the current time",
            ));
        };

        let mut data = BytesMut::with_capacity(bytes_read);
        data.put_slice(&buffer[..bytes_read]);

        let header = RTPHeader::deserialize(&mut data);

        calculate_playout_time(
            arrival_time,
            media_clock_rate,
            data,
            &header,
        );
    }
}

pub async fn rtp_frame_receiver(
    socket: UdpSocket,
    media_clock_rate: u32,
) -> io::Result<()> {
    let mut buffer = [0u8; 1500];

    // let _ = FRAME_OUTPUT.set(Arc::clone(&peer_manager));

    loop {
        let (bytes_read, _) = socket.recv_from(&mut buffer).await?;

        // there's absolutely a bug where if the time switches playout will be messed up!
        // (ex: when there's daylight savings)
        // but the wall clock is "technically" more stable, and less susceptible to skew
        // bet big, take risks, that's the way.

        let now = SystemTime::now();

        let arrival_time = now.duration_since(UNIX_EPOCH);

        let Ok(arrival_time) = arrival_time else {  
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Time error, packet arrived earlier than the current time",
            ));
        };

        // Don't worry too much about copying, we do need to store it anyways
        let mut data = BytesMut::with_capacity(bytes_read);
        data.put_slice(&buffer[..bytes_read]);

        let header = RTPHeader::deserialize(&mut data);

        calculate_playout_time(
            arrival_time,
            media_clock_rate,
            data,
            &header,
        );
    

        //println!("{}: {}", addr.to_string(), bytes_read);
    }
}

fn calculate_playout_time(
    arrival_time: Duration,
    media_clock_rate: u32,
    data: BytesMut,
    rtp_header: &RTPHeader,
) {
    /*
        Calculating Base Playout time:

        M = T * R + offset
        d(n) = Arrival Time of Packet - Header Timestamp
        offset = Min(d(n-w)...d(n))
        base playout time = Timestamp + offset
    */

    // M = T * R + offset
    // don't worry that we're cutting off the bits
    // the method described in Perkin's book uses modulo arithmetic
    let arrival_time = arrival_time.as_millis() as u32 * (media_clock_rate / 1000);

    // d(n) = Arrival Time of Packet - Header Timestamp
    let difference = arrival_time.wrapping_sub(rtp_header.timestamp);

    // offset = Min(d(n-w)...d(n))
    // in the case when arrival time is smaller than timestamp.
    // wraparound comparison is handled here.
}