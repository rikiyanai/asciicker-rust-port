//! 16-track round-robin audio mixer matching C++ `PlyTrack[16]` architecture.
//!
//! Provides [`AudioMixer`] resource for managing 16 dynamic audio channels with
//! per-track and master volume control, and [`PlaySoundEvent`] for ECS-idiomatic
//! sound playback via Bevy's message system.

use bevy::prelude::*;
use bevy_kira_audio::prelude::*;

/// Number of audio tracks matching C++ `PlyTrack[16]`.
pub const PLY_TRACKS: usize = 16;

/// Audio mixer resource with 16-track round-robin assignment.
///
/// Maps to the C++ engine's `PlyTrack[16]` array. Each track corresponds to a
/// named `DynamicAudioChannel` ("track_0" through "track_15"). Sounds are
/// assigned to tracks via round-robin to prevent channel starvation.
///
/// Volume values are stored as linear amplitude (0.0 = silent, 1.0 = full).
/// Conversion to `kira::Decibels` happens at play time.
#[derive(Resource)]
pub struct AudioMixer {
    /// Master volume multiplier (0.0-1.0 amplitude).
    pub master_volume: f32,
    /// Current round-robin position (0..PLY_TRACKS).
    pub track_round_robin: usize,
    /// Per-track volume multipliers (0.0-1.0 amplitude).
    pub track_volumes: [f32; PLY_TRACKS],
}

impl Default for AudioMixer {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            track_round_robin: 0,
            track_volumes: [1.0; PLY_TRACKS],
        }
    }
}

impl AudioMixer {
    /// Returns the channel name for a given track index.
    #[inline]
    pub fn channel_name(track: usize) -> String {
        format!("track_{track}")
    }

    /// Set master volume (clamped to 0.0-1.0).
    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 1.0);
    }

    /// Set per-track volume (clamped to 0.0-1.0).
    pub fn set_track_volume(&mut self, track: usize, volume: f32) {
        if track < PLY_TRACKS {
            self.track_volumes[track] = volume.clamp(0.0, 1.0);
        }
    }

    /// Get the next track index via round-robin and advance the counter.
    pub fn next_track(&mut self) -> usize {
        let track = self.track_round_robin;
        self.track_round_robin = (self.track_round_robin + 1) % PLY_TRACKS;
        track
    }

    /// Compute effective volume for a track: master * track amplitude.
    /// Returns linear amplitude (0.0-1.0).
    pub fn effective_volume(&self, track: usize) -> f32 {
        if track < PLY_TRACKS {
            self.master_volume * self.track_volumes[track]
        } else {
            0.0
        }
    }

    /// Convert linear amplitude (0.0-1.0) to kira Decibels.
    /// 0.0 maps to silence (-60 dB), 1.0 maps to 0 dB (unity gain).
    pub(crate) fn amplitude_to_decibels(amplitude: f32) -> f32 {
        if amplitude <= 0.0 {
            -60.0 // kira::Decibels::SILENCE
        } else {
            20.0 * amplitude.log10()
        }
    }
}

/// Message to request a sound be played on the audio mixer.
///
/// Send via `MessageWriter<PlaySoundEvent>` from any system. The `play_sound_system`
/// will process these each frame, loading assets and playing on round-robin tracks.
///
/// # Fields
/// - `asset_path`: Path to the audio file (e.g., "sounds/step_L.ogg")
/// - `volume`: Optional volume override (0.0-1.0 amplitude). If None, uses track effective volume.
/// - `track`: Optional specific track (0-15). If None, uses round-robin assignment.
#[derive(Message, Debug, Clone)]
pub struct PlaySoundEvent {
    /// Asset path relative to the assets directory.
    pub asset_path: String,
    /// Volume override (0.0-1.0 amplitude). None = use track effective volume.
    pub volume: Option<f32>,
    /// Specific track index (0-15). None = round-robin assignment.
    pub track: Option<usize>,
}

/// System that processes [`PlaySoundEvent`] messages and plays audio on dynamic channels.
///
/// Drains events unconditionally (P7-055 FIX) -- events are consumed regardless of game
/// state to prevent accumulation. This is correct behavior: UI feedback during loading
/// is standard, and bevy_kira_audio silently queues playback for unloaded assets.
pub fn play_sound_system(
    mut sound_events: MessageReader<PlaySoundEvent>,
    dynamic_channels: Res<DynamicAudioChannels>,
    mut mixer: ResMut<AudioMixer>,
    asset_server: Res<AssetServer>,
) {
    for event in sound_events.read() {
        // Determine which track to use
        let track = match event.track {
            Some(t) if t < PLY_TRACKS => t,
            Some(_) => mixer.next_track(), // Invalid track -> fall back to round-robin
            None => mixer.next_track(),
        };

        let channel_name = AudioMixer::channel_name(track);

        // Only play if the channel exists
        if let Some(channel) = dynamic_channels.get_channel(&channel_name) {
            let handle = asset_server.load::<AudioSource>(&event.asset_path);

            // Compute volume: event override or effective (master * track)
            let amplitude = event
                .volume
                .unwrap_or_else(|| mixer.effective_volume(track))
                .clamp(0.0, 1.0);
            let db = AudioMixer::amplitude_to_decibels(amplitude);

            channel.play(handle).with_volume(db);
        } else {
            warn!(
                "Audio channel '{}' not found -- sound '{}' dropped",
                channel_name, event.asset_path
            );
        }
    }
}

/// Startup system to create all 16 dynamic audio channels.
/// Registered by [`AsciickerAudioPlugin`](super::AsciickerAudioPlugin).
pub fn initialize_audio_channels(mut dynamic_channels: ResMut<DynamicAudioChannels>) {
    for i in 0..PLY_TRACKS {
        let name = AudioMixer::channel_name(i);
        dynamic_channels.create_channel(&name);
    }
    info!("Initialized {} dynamic audio channels", PLY_TRACKS);
}
