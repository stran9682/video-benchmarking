use bytes::{Buf, BufMut, BytesMut};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PacketType {
    Unsupported = 0,
    SenderReport = 200,      // RFC 3550, 6.4.1
    SourceDescription = 202, // RFC 3550, 6.5
    Goodbye = 203,           // RFC 3550, 6.6
}

impl PacketType {
    fn from(b: u8) -> Self {
        match b {
            200 => PacketType::SenderReport,      // RFC 3550, 6.4.1
            202 => PacketType::SourceDescription, // RFC 3550, 6.5
            203 => PacketType::Goodbye,           // RFC 3550, 6.6
            _ => PacketType::Unsupported,
        }
    }
}

/*
     0                   1                   2                   3
     0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |V=2|P|    RC   |   PT=SR=200   |             length            |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
*/
pub struct RTCPHeader {
    pub padding: bool,
    pub count: u8,
    pub packet_type: PacketType,
    pub length: u16,
}

impl RTCPHeader {
    pub fn serialize(&self) -> BytesMut {
        // TODO: Adjust this number lol
        let mut buf = BytesMut::with_capacity(4);

        let b0 = (2 << 6) | ((self.padding as u8) << 5) | (self.count << 0);

        buf.put_u8(b0);
        buf.put_u8(self.packet_type as u8);
        buf.put_u16(self.length);

        buf
    }

    pub fn deserialize(packet: &mut BytesMut) -> RTCPHeader {
        let b0 = packet.get_u8();
        //let version = (b0 >> VERSION_SHIFT) & VERSION_MASK;

        let padding = ((b0 >> 5) & 0x1) > 0;
        let count = (b0 >> 0) & 0x1f;
        let packet_type = PacketType::from(packet.get_u8());
        let length = packet.get_u16();

        RTCPHeader {
            padding,
            count,
            packet_type,
            length,
        }
    }
}

pub struct SenderReport {
    pub ssrc: u32,
    pub ntp_time: u64,
    pub rtp_time: u32,
    pub packet_count: u32,
    pub octet_count: u32,
    pub reports: Vec<ReceptionReport>,
}

impl SenderReport {
    pub fn serialize(&self) -> BytesMut {
        /*
         *         0                   1                   2                   3
         *         0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
         *        +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
         * sender |              NTP timestamp, most significant word             |
         * info   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
         *        |             NTP timestamp, least significant word             |
         *        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
         *        |                         RTP timestamp                         |
         *        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
         *        |                     sender's packet count                     |
         *        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
         *        |                      sender's octet count                     |
         *        +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
         * report |                 SSRC_1 (SSRC of first source)                 |
         * block  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
         *   1    | fraction lost |       cumulative number of packets lost       |
         *        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
         *        |           extended highest sequence number received           |
         *        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
         *        |                      interarrival jitter                      |
         *        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
         *        |                         last SR (LSR)                         |
         *        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
         *        |                   delay since last SR (DLSR)                  |
         *        +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
         * report |                 SSRC_2 (SSRC of second source)                |
         * block  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
         *   2    :                               ...                             :
         *        +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
         *        |                  profile-specific extensions                  |
         *        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
         */
        let mut buf = BytesMut::new();

        buf.put_u32(self.ssrc);
        buf.put_u64(self.ntp_time);
        buf.put_u32(self.rtp_time);
        buf.put_u32(self.packet_count);
        buf.put_u32(self.octet_count);

        for report in &self.reports {
            buf.put(report.serialize());
        }
        buf
    }

    pub fn deserialize(packet: &mut BytesMut, report_counts: u8) -> Self {
        let ssrc = packet.get_u32();
        let ntp_time = packet.get_u64();
        let rtp_time = packet.get_u32();
        let packet_count = packet.get_u32();
        let octet_count = packet.get_u32();

        let mut reports = Vec::with_capacity(report_counts as usize);
        for _ in 0..report_counts {
            let reception_report = ReceptionReport::deserialize(packet);
            reports.push(reception_report);
        }

        SenderReport {
            ssrc,
            ntp_time,
            rtp_time,
            packet_count,
            octet_count,
            reports,
        }
    }

    // TODO: actually calculate this the right way
    pub fn length(&self) -> u16 {
        24 + (self.reports.len() * 24) as u16
    }
}

pub struct ReceptionReport {
    pub reportee_ssrc: u32,
    pub fraction_lost: u8,
    pub total_lost: u32,
    pub extended_sequence_number: u32,
    pub jitter: u32,
    pub last_sr_timestamp: u32,
    pub delay_since_last_sr: u32,
}

impl ReceptionReport {
    pub fn serialize(&self) -> BytesMut {
        /*
         *  0                   1                   2                   3
         *  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
         * +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
         * |                              SSRC                             |
         * +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
         * | fraction lost |       cumulative number of packets lost       |
         * +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
         * |           extended highest sequence number received           |
         * +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
         * |                      interarrival jitter                      |
         * +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
         * |                         last SR (LSR)                         |
         * +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
         * |                   delay since last SR (DLSR)                  |
         * +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
         */

        let mut buf = BytesMut::with_capacity(24);

        buf.put_u32(self.reportee_ssrc);

        buf.put_u8(self.fraction_lost);

        buf.put_u8(((self.total_lost >> 16) & 0xFF) as u8);
        buf.put_u8(((self.total_lost >> 8) & 0xFF) as u8);
        buf.put_u8((self.total_lost & 0xFF) as u8);

        buf.put_u32(self.extended_sequence_number);
        buf.put_u32(self.jitter);
        buf.put_u32(self.last_sr_timestamp);
        buf.put_u32(self.delay_since_last_sr);

        buf
    }

    pub fn deserialize(packet: &mut BytesMut) -> Self {
        let reportee_ssrc = packet.get_u32();
        let fraction_lost = packet.get_u8();

        let t0 = packet.get_u8();
        let t1 = packet.get_u8();
        let t2 = packet.get_u8();
        let total_lost = (t2 as u32) | (t1 as u32) << 8 | (t0 as u32) << 16;

        let extended_sequence_number = packet.get_u32();
        let jitter = packet.get_u32();
        let last_sr_timestamp = packet.get_u32();
        let delay_since_last_sr = packet.get_u32();

        ReceptionReport {
            reportee_ssrc,
            fraction_lost,
            total_lost,
            extended_sequence_number,
            jitter,
            last_sr_timestamp,
            delay_since_last_sr,
        }
    }
}
