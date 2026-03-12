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

#[cfg(test)]
mod tests {
    use super::*;

    // --- Default initialization ---

    #[test]
    fn test_default_master_volume_is_one() {
        let mixer = AudioMixer::default();
        assert_eq!(mixer.master_volume, 1.0);
    }

    #[test]
    fn test_default_round_robin_starts_at_zero() {
        let mixer = AudioMixer::default();
        assert_eq!(mixer.track_round_robin, 0);
    }

    #[test]
    fn test_default_all_track_volumes_are_one() {
        let mixer = AudioMixer::default();
        for i in 0..PLY_TRACKS {
            assert_eq!(mixer.track_volumes[i], 1.0, "track {i} should be 1.0");
        }
    }

    #[test]
    fn test_ply_tracks_is_16() {
        assert_eq!(PLY_TRACKS, 16);
    }

    // --- Channel naming ---

    #[test]
    fn test_channel_names() {
        assert_eq!(AudioMixer::channel_name(0), "track_0");
        assert_eq!(AudioMixer::channel_name(15), "track_15");
    }

    // --- Round-robin ---

    #[test]
    fn test_round_robin_cycles_0_through_15() {
        let mut mixer = AudioMixer::default();
        for expected in 0..PLY_TRACKS {
            assert_eq!(mixer.next_track(), expected);
        }
    }

    #[test]
    fn test_round_robin_wraps_at_16() {
        let mut mixer = AudioMixer::default();
        // Exhaust all 16 tracks
        for _ in 0..PLY_TRACKS {
            mixer.next_track();
        }
        // Should wrap back to 0
        assert_eq!(mixer.next_track(), 0);
        assert_eq!(mixer.next_track(), 1);
    }

    #[test]
    fn test_round_robin_full_cycle_twice() {
        let mut mixer = AudioMixer::default();
        for cycle in 0..2 {
            for expected in 0..PLY_TRACKS {
                let track = mixer.next_track();
                assert_eq!(
                    track, expected,
                    "cycle {cycle}, expected track {expected}, got {track}"
                );
            }
        }
    }

    // --- Volume clamping ---

    #[test]
    fn test_set_master_volume_clamps_above_one() {
        let mut mixer = AudioMixer::default();
        mixer.set_master_volume(2.0);
        assert_eq!(mixer.master_volume, 1.0);
    }

    #[test]
    fn test_set_master_volume_clamps_below_zero() {
        let mut mixer = AudioMixer::default();
        mixer.set_master_volume(-0.5);
        assert_eq!(mixer.master_volume, 0.0);
    }

    #[test]
    fn test_set_master_volume_normal() {
        let mut mixer = AudioMixer::default();
        mixer.set_master_volume(0.5);
        assert_eq!(mixer.master_volume, 0.5);
    }

    #[test]
    fn test_set_track_volume_clamps_above_one() {
        let mut mixer = AudioMixer::default();
        mixer.set_track_volume(5, 1.5);
        assert_eq!(mixer.track_volumes[5], 1.0);
    }

    #[test]
    fn test_set_track_volume_clamps_below_zero() {
        let mut mixer = AudioMixer::default();
        mixer.set_track_volume(5, -1.0);
        assert_eq!(mixer.track_volumes[5], 0.0);
    }

    #[test]
    fn test_set_track_volume_out_of_range_ignored() {
        let mut mixer = AudioMixer::default();
        mixer.set_track_volume(PLY_TRACKS, 0.5); // Should be no-op
        // All tracks still at default
        for i in 0..PLY_TRACKS {
            assert_eq!(mixer.track_volumes[i], 1.0);
        }
    }

    // --- Effective volume ---

    #[test]
    fn test_effective_volume_full() {
        let mixer = AudioMixer::default();
        // master=1.0 * track=1.0 = 1.0
        assert_eq!(mixer.effective_volume(0), 1.0);
    }

    #[test]
    fn test_effective_volume_master_half() {
        let mut mixer = AudioMixer::default();
        mixer.set_master_volume(0.5);
        // master=0.5 * track=1.0 = 0.5
        assert!((mixer.effective_volume(0) - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_effective_volume_track_half() {
        let mut mixer = AudioMixer::default();
        mixer.set_track_volume(3, 0.5);
        // master=1.0 * track=0.5 = 0.5
        assert!((mixer.effective_volume(3) - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_effective_volume_both_half() {
        let mut mixer = AudioMixer::default();
        mixer.set_master_volume(0.5);
        mixer.set_track_volume(7, 0.5);
        // 0.5 * 0.5 = 0.25
        assert!((mixer.effective_volume(7) - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn test_effective_volume_muted_master() {
        let mut mixer = AudioMixer::default();
        mixer.set_master_volume(0.0);
        assert_eq!(mixer.effective_volume(0), 0.0);
    }

    #[test]
    fn test_effective_volume_out_of_range_returns_zero() {
        let mixer = AudioMixer::default();
        assert_eq!(mixer.effective_volume(PLY_TRACKS), 0.0);
        assert_eq!(mixer.effective_volume(999), 0.0);
    }

    // --- Amplitude to decibels ---

    #[test]
    fn test_amplitude_to_decibels_full() {
        // 1.0 amplitude = 0 dB
        let db = AudioMixer::amplitude_to_decibels(1.0);
        assert!((db - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_amplitude_to_decibels_half() {
        // 0.5 amplitude ~ -6.02 dB
        let db = AudioMixer::amplitude_to_decibels(0.5);
        assert!((db - (-6.0206)).abs() < 0.01);
    }

    #[test]
    fn test_amplitude_to_decibels_zero_is_silence() {
        let db = AudioMixer::amplitude_to_decibels(0.0);
        assert_eq!(db, -60.0);
    }

    #[test]
    fn test_amplitude_to_decibels_negative_is_silence() {
        let db = AudioMixer::amplitude_to_decibels(-0.1);
        assert_eq!(db, -60.0);
    }

    // --- PlaySoundEvent ---

    #[test]
    fn test_play_sound_event_fields() {
        let event = PlaySoundEvent {
            asset_path: "sounds/step_L.ogg".to_string(),
            volume: Some(0.8),
            track: Some(3),
        };
        assert_eq!(event.asset_path, "sounds/step_L.ogg");
        assert_eq!(event.volume, Some(0.8));
        assert_eq!(event.track, Some(3));
    }

    #[test]
    fn test_play_sound_event_defaults_none() {
        let event = PlaySoundEvent {
            asset_path: "sounds/hit.ogg".to_string(),
            volume: None,
            track: None,
        };
        assert!(event.volume.is_none());
        assert!(event.track.is_none());
    }

    // --- DynamicAudioChannels integration (R13-019 FIX) ---

    #[test]
    fn test_all_16_channels_can_be_created() {
        let mut channels = DynamicAudioChannels::default();
        for i in 0..PLY_TRACKS {
            let name = AudioMixer::channel_name(i);
            channels.create_channel(&name);
        }
        // Verify all 16 channels exist
        for i in 0..PLY_TRACKS {
            let name = AudioMixer::channel_name(i);
            assert!(
                channels.is_channel(&name),
                "Channel '{}' should exist",
                name
            );
        }
    }

    #[test]
    fn test_channel_round_robin_reuse_after_16() {
        // After cycling through all 16 tracks, the 17th event reuses track_0
        let mut mixer = AudioMixer::default();
        for _ in 0..PLY_TRACKS {
            mixer.next_track();
        }
        // 17th track should wrap to 0
        assert_eq!(
            mixer.next_track(),
            0,
            "17th event should reuse track_0 (round-robin wrap)"
        );
    }
}
