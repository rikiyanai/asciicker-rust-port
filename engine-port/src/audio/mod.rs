//! Audio subsystem for the Asciicker engine.
//!
//! Provides a 16-track dynamic audio channel mixer matching the C++ engine's
//! `PlyTrack[16]` architecture, built on top of `bevy_kira_audio`.

pub mod mixer;

use bevy::prelude::*;

pub use mixer::{AudioMixer, PlaySoundEvent};

/// Asciicker audio plugin. Wraps `bevy_kira_audio::AudioPlugin` and provides
/// a 16-track round-robin mixer via [`AudioMixer`].
///
/// Named `AsciickerAudioPlugin` to avoid conflict with `bevy_kira_audio::AudioPlugin`.
pub struct AsciickerAudioPlugin;

impl Plugin for AsciickerAudioPlugin {
    fn build(&self, app: &mut App) {
        // Add bevy_kira_audio as a sub-plugin (provides AudioSource, DynamicAudioChannels, etc.)
        app.add_plugins(bevy_kira_audio::AudioPlugin);

        // Initialize the 16-track mixer resource
        app.init_resource::<AudioMixer>();

        // Register PlaySoundEvent as a Bevy 0.18 Message (NOT add_event)
        app.add_message::<PlaySoundEvent>();

        // Create all 16 dynamic audio channels at startup
        app.add_systems(Startup, mixer::initialize_audio_channels);

        // System that processes PlaySoundEvents and plays audio on dynamic channels
        app.add_systems(Update, mixer::play_sound_system);
    }
}
