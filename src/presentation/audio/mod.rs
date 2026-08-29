use std::io::Cursor;

use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};

const TIME_GATE_TRACK: &[u8] = include_bytes!("../../../Hacking to the Gate.mp3");
const BACKGROUND_VOLUME: f32 = 0.12;

pub(crate) struct EasterEggAudio {
    playback: Option<Playback>,
}

struct Playback {
    player: Player,
    _device_sink: MixerDeviceSink,
}

impl EasterEggAudio {
    pub(crate) const fn new() -> Self {
        Self { playback: None }
    }

    pub(crate) fn play_time_gate(&mut self) -> Result<(), String> {
        if self.playback.is_some() {
            return Ok(());
        }

        let device_sink = DeviceSinkBuilder::open_default_sink()
            .map_err(|error| format!("could not open the default audio device: {error}"))?;
        let decoder = Decoder::builder()
            .with_data(Cursor::new(TIME_GATE_TRACK))
            .with_byte_len(TIME_GATE_TRACK.len() as u64)
            .with_hint("mp3")
            .build()
            .map_err(|error| format!("could not decode the embedded Sim;Time track: {error}"))?;
        let player = Player::connect_new(device_sink.mixer());
        player.set_volume(BACKGROUND_VOLUME);
        player.append(decoder);
        self.playback = Some(Playback {
            player,
            _device_sink: device_sink,
        });
        Ok(())
    }

    pub(crate) fn stop(&mut self) {
        if let Some(playback) = self.playback.take() {
            playback.player.stop();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use rodio::Decoder;

    use super::TIME_GATE_TRACK;

    #[test]
    fn embedded_time_gate_track_is_a_decodable_mp3() {
        let decoder = Decoder::builder()
            .with_data(Cursor::new(TIME_GATE_TRACK))
            .with_byte_len(TIME_GATE_TRACK.len() as u64)
            .with_hint("mp3")
            .build();

        assert!(decoder.is_ok());
    }
}
