pub mod receivers;
pub mod rtp;

#[derive(Debug, Clone, Copy)]
pub enum StreamType {
    Audio,
    Video,
}
