pub mod rtp;
pub mod receivers;

#[derive(Debug, Clone, Copy)]
pub enum StreamType {
    Audio,
    Video,
}

