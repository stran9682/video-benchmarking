use std::{
    io, sync::{Arc, atomic::{AtomicU32, AtomicU64, Ordering::Relaxed}}, time::{Duration, SystemTime}
};

use csv::Writer;
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
    // let mut last_seen_timestamp: u32 = 0;

    let mut wtr = Writer::from_path("data.csv")?;
    let mut samples = 0;

    loop {
        let (bytes_read, _) = socket.recv_from(&mut buffer).await?;

        let now = SystemTime::now();
        let time_since_epoch = now.duration_since(SystemTime::UNIX_EPOCH).unwrap();

        let mut data = BytesMut::with_capacity(bytes_read);
        data.put_slice(&buffer[..bytes_read]);

        let header = RTPHeader::deserialize(&mut data);

        if samples < 3500 {
            wtr.write_record(&[header.ssrc.to_string(), header.timestamp.to_string(), time_since_epoch.as_nanos().to_string()])?;
            samples += 1;
        } else {
            wtr.flush()?;
        }

        // let ntp = rtcp_sender_ntp.load(Relaxed);
        // let timestamp = rtcp_sender_timestamp.load(Relaxed);

        // if  ntp == 0 || last_seen_timestamp == header.timestamp {
        //     continue;
        // }

        // last_seen_timestamp = header.timestamp;

        // let delay_ns = calculate_delay(
        //     time_since_epoch, 
        //     media_clock_rate, 
        //     data, 
        //     &header, 
        //     ntp, 
        //     timestamp
        // );
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
) -> u64 {

    // mostly from: 
    // https://eceweb1.rutgers.edu/~marsic/books/CN/projects/wireshark/ws-project-4.html

    let unit_difference = rtp_header.timestamp.wrapping_sub(rtcp_sender_timestamp);
    let delta_ns = unit_difference as u64 * 1_000_000_000 / media_clock_rate as u64;
    
    let ntp_secs = (rtcp_sender_ntp >> 32) as u64;
    let ntp_frac = (rtcp_sender_ntp & 0xFFFFFFFF) as u64;

    let ntp_frac_ns = (ntp_frac * 1_000_000_000) >> 32;

    let packet_send_time = ((ntp_secs - NTP_TO_UNIX_EPOCH_SECS) * 1_000_000_000) 
                                    + ntp_frac_ns 
                                    + delta_ns;

    let arrival_ns = arrival_time.as_nanos() as u64;
    let delay_ns = arrival_ns.saturating_sub(packet_send_time); 

    // println!("RTP Diff (ticks): {}", unit_difference);
    // println!("Delta (ms): {}", delta_ns as f64 / 1_000_000.0);
    // println!("NTP Base (secs): {}", ntp_secs);
    // println!("NTP Frac: {}", ntp_frac_ns);
    // println!("Packet Send Time (Unix ns): {}", packet_send_time);
    // println!("Arrival (Unix ns): {}", arrival_ns);
    // println!("{}: {}", rtp_header.timestamp, packet_send_time);

    delay_ns
}