use serde::{Deserialize, Serialize};

pub mod receivers;
pub mod signalling;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
enum StreamTypeWithArgs {
    Video { pps: Vec<u8>, sps: Vec<u8> },
    Audio { sample_rate: f64, channels: u32 },
    BenchmarkAudio,
    BenchmarkVideo,
}

#[derive(Serialize, Deserialize, Debug)]
struct ServerArgs {
    signaling_address: String,
    local_rtp_address: String,
    ssrc: u32,
    stream_type: StreamTypeWithArgs,
    peer_signalling_addresses: Vec<String>,
}
