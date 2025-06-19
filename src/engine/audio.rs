#[cfg(feature = "audio")]
use rodio::{Decoder, OutputStream, Sink};
#[cfg(feature = "audio")]
use std::io::Cursor;

#[cfg(feature = "audio")]
pub struct AudioSystem {
    _stream: OutputStream,
    sink: Sink,
}

#[cfg(not(feature = "audio"))]
pub struct AudioSystem;

impl AudioSystem {
    #[cfg(feature = "audio")]
    pub fn new() -> Self {
        let (_stream, handle) = OutputStream::try_default().expect("audio init");
        let sink = Sink::try_new(&handle).expect("sink");
        Self { _stream, sink }
    }

    #[cfg(not(feature = "audio"))]
    pub fn new() -> Self {
        Self
    }

    #[cfg(feature = "audio")]
    pub fn play_bytes(&self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        if let Ok(decoder) = Decoder::new(Cursor::new(bytes.to_vec())) {
            self.sink.append(decoder);
        }
    }

    #[cfg(not(feature = "audio"))]
    pub fn play_bytes(&self, _bytes: &[u8]) {}
}
