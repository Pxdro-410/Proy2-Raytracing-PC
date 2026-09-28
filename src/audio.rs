use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::time::Duration;

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

const MUSIC_VOLUME: f32 = 0.22;
const BUTTON_VOLUME: f32 = 0.65;
const BUTTON_PRESS_DURATION: Duration = Duration::from_millis(100);
const BUTTON_RELEASE_OFFSET: Duration = Duration::from_millis(320);
const BUTTON_RELEASE_DURATION: Duration = Duration::from_millis(180);

/// Reproductor de los sonidos globales de la aplicación.
///
/// Mantiene vivo el `OutputStream` durante toda la ejecución. Si el equipo no
/// tiene una salida de audio disponible, el juego continúa sin sonido.
pub struct Audio {
    _stream: Option<OutputStream>,
    handle: Option<OutputStreamHandle>,
    _soundtrack: Option<Sink>,
}

impl Audio {
    /// Inicia la banda sonora en bucle desde el arranque del programa.
    pub fn load() -> Self {
        match Self::try_load() {
            Ok(audio) => audio,
            Err(error) => {
                eprintln!("Audio desactivado: {error}");
                Self {
                    _stream: None,
                    handle: None,
                    _soundtrack: None,
                }
            }
        }
    }

    /// Reproduce el primer fragmento del WAV: la pulsación del botón.
    pub fn play_button_press(&self) {
        self.play_button_segment(Duration::ZERO, BUTTON_PRESS_DURATION);
    }

    /// Reproduce el segundo fragmento del WAV: la liberación del botón.
    pub fn play_button_release(&self) {
        self.play_button_segment(BUTTON_RELEASE_OFFSET, BUTTON_RELEASE_DURATION);
    }

    /// El archivo de botón reúne ambos sonidos en una sola pista: el clic de
    /// pulsación al inicio y el de liberación al final. Cada segmento se crea
    /// por evento sin interrumpir la música de fondo.
    fn play_button_segment(&self, offset: Duration, duration: Duration) {
        let Some(handle) = &self.handle else {
            return;
        };
        let Ok(file) = File::open(button_sound_path()) else {
            return;
        };
        let Ok(effect) = Decoder::new(BufReader::new(file)) else {
            return;
        };
        let Ok(sink) = Sink::try_new(handle) else {
            return;
        };

        sink.set_volume(BUTTON_VOLUME);
        sink.append(effect.skip_duration(offset).take_duration(duration));
        // El mezclador conserva el sonido aunque el `Sink` local se libere.
        sink.detach();
    }

    fn try_load() -> Result<Self, String> {
        let (stream, handle) =
            OutputStream::try_default().map_err(|error| format!("salida de audio: {error}"))?;
        let file = File::open(soundtrack_path())
            .map_err(|error| format!("no se pudo abrir soundtrack.mp3: {error}"))?;
        let soundtrack = Decoder::new_looped(BufReader::new(file))
            .map_err(|error| format!("no se pudo decodificar soundtrack.mp3: {error}"))?;
        let sink = Sink::try_new(&handle)
            .map_err(|error| format!("no se pudo crear el reproductor musical: {error}"))?;

        sink.set_volume(MUSIC_VOLUME);
        sink.append(soundtrack);

        Ok(Self {
            _stream: Some(stream),
            handle: Some(handle),
            _soundtrack: Some(sink),
        })
    }
}

fn sound_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("sound")
}

fn soundtrack_path() -> PathBuf {
    sound_directory().join("soundtrack.mp3")
}

fn button_sound_path() -> PathBuf {
    sound_directory().join("button_click_and_release.wav")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_audio_assets_are_present() {
        assert!(soundtrack_path().is_file());
        assert!(button_sound_path().is_file());
    }

    #[test]
    fn bundled_audio_assets_can_be_decoded() {
        let soundtrack = File::open(soundtrack_path()).expect("debe abrir la banda sonora");
        Decoder::new(BufReader::new(soundtrack)).expect("debe decodificar el MP3");

        let button = File::open(button_sound_path()).expect("debe abrir el sonido de botón");
        Decoder::new(BufReader::new(button)).expect("debe decodificar el WAV");
    }
}
