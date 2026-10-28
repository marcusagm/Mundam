use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::Manager;
use tracing::info;

use hound::{WavSpec, WavWriter};
use midly::Smf;
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};

/// Locates a SoundFont (.sf2) file in the application's resources directory.
pub fn locate_soundfont(app_handle: Option<&tauri::AppHandle>) -> Option<PathBuf> {
    // Priority 1: User's app local data dir / soundfonts
    if let Some(app) = app_handle {
        if let Ok(data_dir) = app.path().app_local_data_dir() {
            let sf_dir = data_dir.join("soundfonts");
            if let Ok(entries) = std::fs::read_dir(&sf_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().is_some_and(|e| e == "sf2") {
                        return Some(path);
                    }
                }
            }
        }
    }

    // Priority 2: Bundled Tauri resources (the installer places these beside
    // the executable on Windows, under the app's Resources directory on macOS).
    if let Some(app) = app_handle {
        if let Ok(resource_dir) = app.path().resource_dir() {
            if let Some(path) = find_soundfont(&resource_dir.join("resources/soundfonts")) {
                return Some(path);
            }
        }
    }

    // Priority 3: Project resources during local development.
    let local_resources = Path::new("resources/soundfonts");
    find_soundfont(local_resources)
}

fn find_soundfont(directory: &Path) -> Option<PathBuf> {
    std::fs::read_dir(directory)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.is_file() && path.extension().is_some_and(|e| e == "sf2"))
}

/// Parses a MIDI file and returns its length in seconds.
/// Does not require a SoundFont, making it useful for probing.
pub fn get_midi_length(midi_path: &Path) -> Result<f64, String> {
    let midi_data = std::fs::read(midi_path).map_err(|e| format!("Failed to read MIDI: {}", e))?;
    let midi_file = rustysynth::MidiFile::new(&mut std::io::Cursor::new(midi_data))
        .map_err(|e| format!("Failed to load MidiFile: {}", e))?;
    Ok(midi_file.get_length())
}

/// Renders a MIDI file to a WAV file using a SoundFont.
///
/// Uses `midly` to parse the MIDI and `rustysynth` to synthesize the audio.
pub async fn render_midi_to_wav(
    midi_path: &Path,
    wav_path: &Path,
    app_handle: Option<&tauri::AppHandle>,
) -> Result<(), String> {
    info!("Starting MIDI to WAV synthesis for {:?}", midi_path);

    let sf2_path = locate_soundfont(app_handle).ok_or_else(|| {
        "No SoundFont (.sf2) found! Please place a .sf2 file in resources/soundfonts/ or in the app's local data directory under 'soundfonts'."
            .to_string()
    })?;

    info!("Using SoundFont at {:?}", sf2_path);

    let sf2_path_owned = sf2_path.clone();
    let midi_path_owned = midi_path.to_path_buf();
    let wav_path_owned = wav_path.to_path_buf();

    tokio::task::spawn_blocking(move || -> Result<(), String> {
        // 1. Load SoundFont
        let mut sf2_file =
            File::open(&sf2_path_owned).map_err(|e| format!("Failed to open SoundFont: {}", e))?;
        let sound_font = Arc::new(
            SoundFont::new(&mut sf2_file).map_err(|e| format!("Invalid SoundFont: {}", e))?,
        );

        // 2. Setup Synthesizer
        let sample_rate = 44100;
        let settings = SynthesizerSettings::new(sample_rate as i32);
        let synthesizer = Synthesizer::new(&sound_font, &settings)
            .map_err(|e| format!("Failed to create synthesizer: {}", e))?;

        // 3. Read MIDI file
        let midi_data =
            std::fs::read(&midi_path_owned).map_err(|e| format!("Failed to read MIDI: {}", e))?;
        let _smf = Smf::parse(&midi_data).map_err(|e| format!("Failed to parse MIDI: {}", e))?;

        // 4. Setup Sequencer
        let mut sequencer = rustysynth::MidiFileSequencer::new(synthesizer);
        let midi_file = rustysynth::MidiFile::new(&mut std::io::Cursor::new(midi_data))
            .map_err(|e| format!("Failed to load MidiFile: {}", e))?;
        let midi_file = Arc::new(midi_file);
        sequencer.play(&midi_file, false);

        // 5. Setup Output WAV
        let spec = WavSpec {
            channels: 2,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = WavWriter::create(&wav_path_owned, spec)
            .map_err(|e| format!("Failed to create WAV: {}", e))?;

        // 6. Synthesize loop
        let chunk_size = sample_rate as usize; // 1 second chunks
        let mut left_buffer = vec![0.0f32; chunk_size];
        let mut right_buffer = vec![0.0f32; chunk_size];

        while !sequencer.end_of_sequence() {
            sequencer.render(&mut left_buffer[..], &mut right_buffer[..]);

            for i in 0..chunk_size {
                let l = (left_buffer[i].clamp(-1.0, 1.0) * 32767.0) as i16;
                let r = (right_buffer[i].clamp(-1.0, 1.0) * 32767.0) as i16;

                writer
                    .write_sample(l)
                    .map_err(|e| format!("WAV write error: {}", e))?;
                writer
                    .write_sample(r)
                    .map_err(|e| format!("WAV write error: {}", e))?;
            }
        }

        writer
            .finalize()
            .map_err(|e| format!("Failed to finalize WAV: {}", e))?;
        info!("MIDI synthesis completed to {:?}", wav_path_owned);
        Ok(())
    })
    .await
    .map_err(|e| format!("Thread panicked: {}", e))??;

    Ok(())
}
