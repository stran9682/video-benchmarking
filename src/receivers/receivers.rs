use std::{
    io, sync::{Arc, atomic::{AtomicU32, AtomicU64, Ordering::Relaxed}}, time::{Duration, SystemTime, UNIX_EPOCH}
};

use bytes::{BufMut, BytesMut};
use tokio::net::UdpSocket;

use crate::rtp::{rtcp::{PacketType, RTCPHeader, SenderReport}, rtp_header::RTPHeader};

pub async fn rtcp_receiver(
    socket: UdpSocket, 
    rtcp_sender_ntp: Arc<AtomicU64>, 
    rtcp_sender_timestamp: Arc<AtomicU32>
) -> io::Result<()> {
    let mut buffer = [0u8; 1500];

    loop {
        let (bytes_read, _) = socket.recv_from(&mut buffer).await?;

        let mut packet = BytesMut::with_capacity(bytes_read);
        packet.put(&buffer[..bytes_read]);

        while packet.len() > 0 {
            let rtcp_header = RTCPHeader::deserialize(&mut packet);

            match rtcp_header.packet_type {
                PacketType::SenderReport => {
                    let sender_report = SenderReport::deserialize(&mut packet, rtcp_header.count);

                    rtcp_sender_ntp.store(sender_report.ntp_time, Relaxed);
                    rtcp_sender_timestamp.store(sender_report.rtp_time, Relaxed);
                }
                _ => {}
            }
        }
    }
}

pub async fn rtp_receiver(socket: UdpSocket, media_clock_rate: u32) -> io::Result<()> {
    let rtcp_sender_ntp = Arc::new(AtomicU64::new(0));
    let rtcp_sender_timestamp = Arc::new(AtomicU32::new(0));

    // RTCP stuff, don't mind the mess
    let addr = format!("{}:{}", socket.local_addr()?.ip(), socket.local_addr()?.port() + 1);
    let rtcp_socket = UdpSocket::bind(addr).await?;
    let ntp_clone = Arc::clone(&rtcp_sender_ntp);
    let timestamp_clone = Arc::clone(&rtcp_sender_timestamp);

    tokio::spawn(async move {
        rtcp_receiver(rtcp_socket, ntp_clone, timestamp_clone).await
    });

    let mut buffer = [0u8; 1500];

    loop {
        let (bytes_read, _) = socket.recv_from(&mut buffer).await?;

        let now = SystemTime::now();
        let time_since_epoch = now.duration_since(SystemTime::UNIX_EPOCH).unwrap();

        let mut data = BytesMut::with_capacity(bytes_read);
        data.put_slice(&buffer[..bytes_read]);

        let header = RTPHeader::deserialize(&mut data);

        calculate_delay(
            time_since_epoch, 
            media_clock_rate, 
            data, 
            &header, 
            rtcp_sender_ntp.load(Relaxed), 
            rtcp_sender_timestamp.load(Relaxed)
        );
    }
}

const NTP_TO_UNIX_EPOCH_SECS: u64 = 2_208_988_800;

fn calculate_delay(
    arrival_time: Duration,
    media_clock_rate: u32,
    data: BytesMut,
    rtp_header: &RTPHeader,
    rtcp_sender_ntp: u64, 
    rtcp_sender_timestamp: u32
) {

    // mostly from: 
    // https://stackoverflow.com/questions/71296697/how-to-calculate-delay-in-rtp-packets-using-rtp-time-and-ntp-time-from-rtcp

    let receiver_ntp= get_ntp_time(arrival_time);
    let packet_send_time = ((rtp_header.timestamp - rtcp_sender_timestamp) / media_clock_rate) as u64 + rtcp_sender_ntp;
    let delay = receiver_ntp - packet_send_time;

    let seconds = (delay >> 32) as u64;
    let fraction = (delay & 0xFFFFFFFF) as u64;
    let nanos_per_frac = 1_000_000_000u64;
    let nanoseconds = (fraction as u128 * nanos_per_frac as u128 >> 32) as u64;
    let unix_seconds = seconds - NTP_TO_UNIX_EPOCH_SECS;
    let delay_ns = unix_seconds * 1_000_000_000 + nanoseconds;
}

fn get_ntp_time(time_since_epoch: Duration) -> u64 {
    let seconds = time_since_epoch.as_secs() + NTP_TO_UNIX_EPOCH_SECS;
    let fraction =
        ((time_since_epoch.subsec_micros() + 1) as f64 * (1u64 << 32) as f64 * 1.0e-6) as u32;
    let ntp = seconds << 32 | (fraction as u64);

    ntp
}