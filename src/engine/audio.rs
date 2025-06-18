use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;

pub struct AudioSystem {
    _stream: OutputStream,
    sink: Sink,
}

impl AudioSystem {
    pub fn new() -> Self {
        let (_stream, handle) = OutputStream::try_default().expect("audio init");
        let sink = Sink::try_new(&handle).expect("sink");
        Self { _stream, sink }
    }

    pub fn play_bytes(&self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        if let Ok(decoder) = Decoder::new(Cursor::new(bytes.to_vec())) {
            self.sink.append(decoder);
        }
    }
}
