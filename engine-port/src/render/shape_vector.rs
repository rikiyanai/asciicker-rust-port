//! Alex Harri shape-vector glyph matching with the upstream default alphabet.
//!
//! This uses the real `default.json` alphabet metadata:
//! - 95 printable ASCII glyph vectors
//! - 6 internal sampling circles
//! - 10 external sampling circles
//! - directional crunch affects-mapping
//!
//! The runtime path stays CPU-side and feeds the existing `GlyphSelector`
//! extension point at resolve time.

use std::collections::BTreeSet;
use std::num::NonZeroUsize;
use std::sync::OnceLock;

use bevy::prelude::*;
use image::{RgbaImage, load_from_memory};
use kiddo::KdTree;
use kiddo::float::distance::SquaredEuclidean;
use lru::LruCache;
use serde::Deserialize;

use crate::render::material::Material;
use crate::render::quantize::rgb555_to_rgb888;
use crate::render::resolve_bridge::GlyphSelector;
use crate::render::sample_buffer::{SampleBuffer, spare_bits};

const DEFAULT_ALPHABET_JSON: &str = include_str!("alphabets/default.json");
const RUNTIME_FONT_PNG: &[u8] = include_bytes!("../../assets/fonts/cp437_10x16.png");
const GOLDEN_ANGLE: f32 = 3.883_222;
const DEFAULT_SAMPLING_QUALITY: usize = 8;
const RUNTIME_FONT_CHAR_WIDTH: u32 = 10;
const RUNTIME_FONT_CHAR_HEIGHT: u32 = 16;

#[derive(Debug, Clone, Copy)]
pub struct CharacterEntry {
    pub glyph: u8,
    pub vector: [f32; 6],
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SamplingPoint {
    x: f32,
    y: f32,
}

#[derive(Debug, Clone)]
pub struct ShapeVectorAlphabetData {
    entries: Vec<CharacterEntry>,
    sampling_points: [SamplingPoint; 6],
    external_points: Vec<SamplingPoint>,
    affects_mapping: [Vec<usize>; 6],
    circle_radius: f32,
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShapeVectorAlphabetId {
    Default,
    Minimal,
}

impl ShapeVectorAlphabetId {
    pub fn next(self) -> Self {
        match self {
            Self::Default => Self::Minimal,
            Self::Minimal => Self::Default,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Minimal => "minimal",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeVectorMode {
    OriginalOnly,
    Combined,
    HarriPriority,
}

impl ShapeVectorMode {
    pub fn next(self) -> Self {
        match self {
            Self::OriginalOnly => Self::Combined,
            Self::Combined => Self::HarriPriority,
            Self::HarriPriority => Self::OriginalOnly,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::OriginalOnly => "original_only",
            Self::Combined => "combined",
            Self::HarriPriority => "harri_priority",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "original_only" | "original" => Some(Self::OriginalOnly),
            "combined" | "original_edges" => Some(Self::Combined),
            "harri_priority" | "harri" => Some(Self::HarriPriority),
            _ => None,
        }
    }
}

#[derive(Resource, Debug)]
pub struct ShapeVectorAlphabetRegistry {
    default: ShapeVectorAlphabetData,
    minimal: ShapeVectorAlphabetData,
}

#[derive(Deserialize)]
struct AlphabetJson {
    metadata: AlphabetMetadataJson,
    characters: Vec<CharacterJson>,
}

#[derive(Deserialize)]
struct AlphabetMetadataJson {
    #[serde(rename = "samplingConfig")]
    sampling_config: SamplingConfigJson,
}

#[derive(Deserialize)]
struct SamplingConfigJson {
    points: Vec<PointJson>,
    #[serde(rename = "externalPoints", default)]
    external_points: Vec<PointJson>,
    #[serde(rename = "affectsMapping", default)]
    affects_mapping: Vec<Vec<usize>>,
    #[serde(rename = "circleRadius")]
    circle_radius: f32,
}

#[derive(Deserialize)]
struct PointJson {
    x: f32,
    y: f32,
}

#[derive(Deserialize)]
struct CharacterJson {
    char: String,
    vector: [f32; 6],
}

fn default_alphabet() -> &'static ShapeVectorAlphabetData {
    static DEFAULT_ALPHABET: OnceLock<ShapeVectorAlphabetData> = OnceLock::new();
    DEFAULT_ALPHABET.get_or_init(parse_default_alphabet)
}

fn parse_default_alphabet() -> ShapeVectorAlphabetData {
    let parsed: AlphabetJson =
        serde_json::from_str(DEFAULT_ALPHABET_JSON).expect("default Alex Harri alphabet JSON");

    let entries = build_runtime_font_entries(&parsed.characters).unwrap_or_else(|| {
        parsed
            .characters
            .iter()
            .map(|entry| {
                let glyph = entry
                    .char
                    .bytes()
                    .next()
                    .expect("alphabet character must be a single-byte ASCII glyph");
                CharacterEntry {
                    glyph,
                    vector: entry.vector,
                }
            })
            .collect::<Vec<_>>()
    });

    let sampling_points = parsed
        .metadata
        .sampling_config
        .points
        .into_iter()
        .map(|p| SamplingPoint { x: p.x, y: p.y })
        .collect::<Vec<_>>()
        .try_into()
        .expect("default alphabet must define exactly 6 internal sampling points");

    let external_points = parsed
        .metadata
        .sampling_config
        .external_points
        .into_iter()
        .map(|p| SamplingPoint { x: p.x, y: p.y })
        .collect::<Vec<_>>();

    let affects_mapping = parsed
        .metadata
        .sampling_config
        .affects_mapping
        .try_into()
        .expect("default alphabet must define exactly 6 affects mappings");

    ShapeVectorAlphabetData {
        entries,
        sampling_points,
        external_points,
        affects_mapping,
        circle_radius: parsed.metadata.sampling_config.circle_radius,
    }
}

impl Default for ShapeVectorAlphabetRegistry {
    fn default() -> Self {
        let default = default_alphabet().clone();
        let minimal = build_minimal_alphabet(&default);
        Self { default, minimal }
    }
}

impl ShapeVectorAlphabetRegistry {
    pub fn get(&self, id: ShapeVectorAlphabetId) -> &ShapeVectorAlphabetData {
        match id {
            ShapeVectorAlphabetId::Default => &self.default,
            ShapeVectorAlphabetId::Minimal => &self.minimal,
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct ShapeVectorGlyphCandidates {
    pub enabled: bool,
    pub glyphs: BTreeSet<u8>,
}

impl Default for ShapeVectorGlyphCandidates {
    fn default() -> Self {
        Self {
            enabled: false,
            glyphs: minimal_candidate_set(),
        }
    }
}

impl ShapeVectorGlyphCandidates {
    pub fn is_active(&self) -> bool {
        self.enabled && !self.glyphs.is_empty()
    }

    pub fn restore_minimal(&mut self) {
        self.enabled = true;
        self.glyphs = minimal_candidate_set();
    }

    pub fn signature(&self) -> u64 {
        custom_glyph_signature(&self.glyphs)
    }

    pub fn build_alphabet(
        &self,
        base: &ShapeVectorAlphabetData,
    ) -> Option<ShapeVectorAlphabetData> {
        if !self.is_active() {
            return None;
        }

        let image = runtime_font_image();
        let mut entries = self
            .glyphs
            .iter()
            .copied()
            .map(|glyph| CharacterEntry {
                glyph,
                vector: sample_runtime_font_glyph(
                    image,
                    glyph,
                    &base.sampling_points,
                    base.circle_radius,
                ),
            })
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.glyph);

        Some(ShapeVectorAlphabetData {
            entries,
            sampling_points: base.sampling_points,
            external_points: base.external_points.clone(),
            affects_mapping: base.affects_mapping.clone(),
            circle_radius: base.circle_radius,
        })
    }
}

fn minimal_candidate_set() -> BTreeSet<u8> {
    [b' ', b'.', b':', b'-', b'=', b'+', b'*', b'#', b'%', b'@']
        .into_iter()
        .collect()
}

pub fn alphabet_signature(id: ShapeVectorAlphabetId) -> u64 {
    match id {
        ShapeVectorAlphabetId::Default => 0xD_EFA0_17,
        ShapeVectorAlphabetId::Minimal => 0x1E55,
    }
}

fn custom_glyph_signature(glyphs: &BTreeSet<u8>) -> u64 {
    let mut hash = 0xC057_0B57_5E7_u64;
    for &glyph in glyphs {
        hash = hash.rotate_left(5) ^ u64::from(glyph);
        hash = hash.wrapping_mul(0x100_0000_01B3);
    }
    hash ^ glyphs.len() as u64
}

fn build_minimal_alphabet(default: &ShapeVectorAlphabetData) -> ShapeVectorAlphabetData {
    let wanted: BTreeSet<u8> = [b' ', b'.', b':', b'-', b'=', b'+', b'*', b'#', b'%', b'@']
        .into_iter()
        .collect();
    let entries = default
        .entries
        .iter()
        .copied()
        .filter(|entry| wanted.contains(&entry.glyph))
        .collect::<Vec<_>>();
    ShapeVectorAlphabetData {
        entries,
        sampling_points: default.sampling_points,
        external_points: default.external_points.clone(),
        affects_mapping: default.affects_mapping.clone(),
        circle_radius: default.circle_radius,
    }
}

fn build_runtime_font_entries(characters: &[CharacterJson]) -> Option<Vec<CharacterEntry>> {
    let image = load_from_memory(RUNTIME_FONT_PNG).ok()?.to_rgba8();
    let alphabet_json: AlphabetJson = serde_json::from_str(DEFAULT_ALPHABET_JSON).ok()?;
    let points: [SamplingPoint; 6] = alphabet_json
        .metadata
        .sampling_config
        .points
        .into_iter()
        .map(|p| SamplingPoint { x: p.x, y: p.y })
        .collect::<Vec<_>>()
        .try_into()
        .ok()?;
    let circle_radius = alphabet_json.metadata.sampling_config.circle_radius;

    Some(
        characters
            .iter()
            .map(|entry| {
                let glyph = entry
                    .char
                    .bytes()
                    .next()
                    .expect("alphabet character must be a single-byte ASCII glyph");
                CharacterEntry {
                    glyph,
                    vector: sample_runtime_font_glyph(&image, glyph, &points, circle_radius),
                }
            })
            .collect(),
    )
}

fn sample_runtime_font_glyph(
    image: &RgbaImage,
    glyph: u8,
    points: &[SamplingPoint; 6],
    circle_radius: f32,
) -> [f32; 6] {
    let glyph_x = (glyph as u32 % 16) * RUNTIME_FONT_CHAR_WIDTH;
    let glyph_y = (glyph as u32 / 16) * RUNTIME_FONT_CHAR_HEIGHT;
    let mut vector = [0.0f32; 6];
    for (idx, point) in points.iter().copied().enumerate() {
        vector[idx] = sample_runtime_font_region(image, glyph_x, glyph_y, point, circle_radius, 24);
    }
    vector
}

fn runtime_font_image() -> &'static RgbaImage {
    static RUNTIME_FONT_IMAGE: OnceLock<RgbaImage> = OnceLock::new();
    RUNTIME_FONT_IMAGE.get_or_init(|| {
        load_from_memory(RUNTIME_FONT_PNG)
            .expect("runtime font png")
            .to_rgba8()
    })
}

fn sample_runtime_font_region(
    image: &RgbaImage,
    glyph_x: u32,
    glyph_y: u32,
    point: SamplingPoint,
    circle_radius: f32,
    quality: usize,
) -> f32 {
    let center_x = glyph_x as f32 + point.x * RUNTIME_FONT_CHAR_WIDTH as f32;
    let center_y = glyph_y as f32 + point.y * RUNTIME_FONT_CHAR_HEIGHT as f32;
    let radius = circle_radius * RUNTIME_FONT_CHAR_WIDTH as f32;
    let mut total = 0.0f32;

    for i in 0..quality {
        let radial = (((i as f32) + 0.5) / quality as f32).sqrt() * radius;
        let angle = GOLDEN_ANGLE * i as f32;
        let px = (center_x + radial * angle.cos()).clamp(
            glyph_x as f32,
            (glyph_x + RUNTIME_FONT_CHAR_WIDTH - 1) as f32,
        );
        let py = (center_y + radial * angle.sin()).clamp(
            glyph_y as f32,
            (glyph_y + RUNTIME_FONT_CHAR_HEIGHT - 1) as f32,
        );
        let pixel = image.get_pixel(px.round() as u32, py.round() as u32);
        let r = pixel[0] as f32 / 255.0;
        let g = pixel[1] as f32 / 255.0;
        let b = pixel[2] as f32 / 255.0;
        let a = pixel[3] as f32 / 255.0;
        total += (0.2126 * r + 0.7152 * g + 0.0722 * b) * a;
    }

    total / quality as f32
}

fn quantize_to_key(v: &[f32; 6]) -> u32 {
    let mut key: u32 = 0;
    for &component in v {
        let q = ((component * 32.0).floor() as u32).min(31);
        key = (key << 5) | q;
    }
    key
}

fn crunch_vector(v: &mut [f32; 6], exponent: f32) {
    let max = v.iter().copied().fold(0.0f32, f32::max);
    if max <= 1e-6 {
        return;
    }
    for component in v.iter_mut() {
        let normalized = *component / max;
        let enhanced = normalized.powf(exponent);
        *component = enhanced * max;
    }
}

fn directional_crunch_vector(
    vector: &mut [f32; 6],
    external_vector: &[f32],
    affects_mapping: &[Vec<usize>; 6],
    exponent: f32,
) {
    for idx in 0..vector.len() {
        let value = vector[idx];
        let mut context_value = 0.0f32;
        for &external_idx in &affects_mapping[idx] {
            if let Some(&external) = external_vector.get(external_idx) {
                context_value = context_value.max(external);
            }
        }

        if context_value <= value || context_value <= 1e-6 {
            continue;
        }

        let normalized = value / context_value;
        let enhanced = normalized.powf(exponent);
        vector[idx] = enhanced * context_value;
    }
}

fn vector_contrast(vector: &[f32; 6]) -> f32 {
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for &component in vector {
        min = min.min(component);
        max = max.max(component);
    }
    (max - min).max(0.0)
}

fn effective_distance_threshold(
    base_threshold: f32,
    contrast: f32,
    boost: f32,
    adaptive_enabled: bool,
) -> f32 {
    if adaptive_enabled {
        (base_threshold + contrast.max(0.0) * boost.max(0.0)).min(4.0)
    } else {
        base_threshold
    }
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct ShapeVectorConfig {
    pub mode: ShapeVectorMode,
    pub alphabet: ShapeVectorAlphabetId,
    pub global_crunch_exponent: f32,
    pub directional_crunch_exponent: f32,
    pub distance_threshold: f32,
    pub contrast_adaptive_threshold_boost: f32,
    pub structural_fallback_distance_threshold: f32,
    pub structural_fallback_contrast_threshold: u16,
    pub sampling_quality: usize,
    pub enable_global_crunch: bool,
    pub enable_directional_crunch: bool,
    pub enable_contrast_adaptive_threshold: bool,
    pub enable_structural_fallback: bool,
}

impl Default for ShapeVectorConfig {
    fn default() -> Self {
        Self {
            mode: ShapeVectorMode::Combined,
            alphabet: ShapeVectorAlphabetId::Default,
            global_crunch_exponent: 2.5,
            directional_crunch_exponent: 6.0,
            distance_threshold: 0.08,
            contrast_adaptive_threshold_boost: 0.25,
            structural_fallback_distance_threshold: 0.22,
            structural_fallback_contrast_threshold: 96,
            sampling_quality: DEFAULT_SAMPLING_QUALITY,
            enable_global_crunch: true,
            enable_directional_crunch: true,
            enable_contrast_adaptive_threshold: false,
            enable_structural_fallback: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeVectorSkipReason {
    Clear,
    Underwater,
    DistanceThreshold,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ShapeVectorDecision {
    pub glyph: Option<u8>,
    pub candidate_glyph: Option<u8>,
    pub distance: Option<f32>,
    pub skip_reason: Option<ShapeVectorSkipReason>,
}

impl ShapeVectorDecision {
    fn skipped(skip_reason: ShapeVectorSkipReason, distance: Option<f32>) -> Self {
        Self {
            glyph: None,
            candidate_glyph: None,
            distance,
            skip_reason: Some(skip_reason),
        }
    }

    fn accepted(glyph: u8, distance: f32) -> Self {
        Self {
            glyph: Some(glyph),
            candidate_glyph: Some(glyph),
            distance: Some(distance),
            skip_reason: None,
        }
    }

    fn threshold_rejected(glyph: u8, distance: f32) -> Self {
        Self {
            glyph: None,
            candidate_glyph: Some(glyph),
            distance: Some(distance),
            skip_reason: Some(ShapeVectorSkipReason::DistanceThreshold),
        }
    }

    pub fn resolve_fallback() -> Self {
        Self::default()
    }
}

#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct ShapeVectorFrameStats {
    pub total_cells: u32,
    pub semantic_gate_cells: u32,
    pub clear_skip_cells: u32,
    pub underwater_skip_cells: u32,
    pub threshold_skip_cells: u32,
    pub selector_match_cells: u32,
    pub selector_override_cells: u32,
    pub resolve_fallback_cells: u32,
    pub fallback_space_cells: u32,
    pub fallback_structural_cells: u32,
    pub final_space_cells: u32,
    pub final_non_space_cells: u32,
    pub colored_space_cells: u32,
    pub matched_distance_count: u32,
    pub threshold_distance_count: u32,
    pub matched_distance_sum: f32,
    pub threshold_distance_sum: f32,
    pub matched_distance_max: f32,
    pub threshold_distance_max: f32,
}

impl ShapeVectorFrameStats {
    pub fn begin_frame(&mut self, total_cells: usize) {
        *self = Self {
            total_cells: total_cells as u32,
            ..Self::default()
        };
    }

    pub fn note_selection(
        &mut self,
        decision: ShapeVectorDecision,
        resolve_glyph: u8,
        final_glyph: u8,
        fg_rgb: [u8; 3],
        bg_rgb: [u8; 3],
    ) {
        match decision.skip_reason {
            Some(ShapeVectorSkipReason::Clear) => self.clear_skip_cells += 1,
            Some(ShapeVectorSkipReason::Underwater) => self.underwater_skip_cells += 1,
            Some(ShapeVectorSkipReason::DistanceThreshold) => self.threshold_skip_cells += 1,
            None => {}
        }

        if let Some(distance) = decision.distance {
            if decision.skip_reason == Some(ShapeVectorSkipReason::DistanceThreshold) {
                self.threshold_distance_count += 1;
                self.threshold_distance_sum += distance;
                self.threshold_distance_max = self.threshold_distance_max.max(distance);
            } else {
                self.matched_distance_count += 1;
                self.matched_distance_sum += distance;
                self.matched_distance_max = self.matched_distance_max.max(distance);
            }
        }

        if decision.glyph.is_some() {
            self.selector_match_cells += 1;
            if final_glyph != resolve_glyph {
                self.selector_override_cells += 1;
            }
        } else {
            self.resolve_fallback_cells += 1;
            if final_glyph == b' ' {
                self.fallback_space_cells += 1;
            } else {
                self.fallback_structural_cells += 1;
            }
        }

        if final_glyph == b' ' {
            self.final_space_cells += 1;
            if !rgb_is_black(bg_rgb) || fg_rgb != bg_rgb {
                self.colored_space_cells += 1;
            }
        } else {
            self.final_non_space_cells += 1;
        }
    }

    pub fn percent_of_total(&self, value: u32) -> f32 {
        if self.total_cells == 0 {
            0.0
        } else {
            value as f32 * 100.0 / self.total_cells as f32
        }
    }

    pub fn avg_matched_distance(&self) -> f32 {
        if self.matched_distance_count == 0 {
            0.0
        } else {
            self.matched_distance_sum / self.matched_distance_count as f32
        }
    }

    pub fn avg_threshold_distance(&self) -> f32 {
        if self.threshold_distance_count == 0 {
            0.0
        } else {
            self.threshold_distance_sum / self.threshold_distance_count as f32
        }
    }
}

fn rgb_is_black(rgb: [u8; 3]) -> bool {
    rgb == [0, 0, 0]
}

#[derive(Resource)]
pub struct ShapeVectorMatcher {
    active_alphabet: ShapeVectorAlphabetId,
    active_signature: u64,
    tree: KdTree<f32, 6>,
    cache: LruCache<u32, (u8, f32)>,
    entries: Vec<CharacterEntry>,
    cache_hits: u64,
    cache_misses: u64,
}

impl ShapeVectorMatcher {
    pub fn new_default() -> Self {
        Self::new(ShapeVectorAlphabetId::Default, &default_alphabet().entries)
    }

    pub fn new(active_alphabet: ShapeVectorAlphabetId, characters: &[CharacterEntry]) -> Self {
        Self::new_with_signature(
            active_alphabet,
            alphabet_signature(active_alphabet),
            characters,
        )
    }

    pub fn new_with_signature(
        active_alphabet: ShapeVectorAlphabetId,
        active_signature: u64,
        characters: &[CharacterEntry],
    ) -> Self {
        let mut tree = KdTree::<f32, 6>::new();
        for (idx, entry) in characters.iter().enumerate() {
            tree.add(&entry.vector, idx as u64);
        }

        Self {
            active_alphabet,
            active_signature,
            tree,
            cache: LruCache::new(NonZeroUsize::new(8192).unwrap()),
            entries: characters.to_vec(),
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    pub fn find_glyph_with_distance(&mut self, vector: [f32; 6]) -> (u8, f32) {
        let key = quantize_to_key(&vector);
        if let Some(&(glyph, distance)) = self.cache.get(&key) {
            self.cache_hits += 1;
            return (glyph, distance);
        }

        self.cache_misses += 1;
        let nearest = self.tree.nearest_one::<SquaredEuclidean>(&vector);
        let glyph = self.entries[nearest.item as usize].glyph;
        let distance = nearest.distance;
        self.cache.put(key, (glyph, distance));
        (glyph, distance)
    }

    pub fn find_glyph(&mut self, vector: [f32; 6]) -> u8 {
        self.find_glyph_with_distance(vector).0
    }

    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
        self.cache_hits = 0;
        self.cache_misses = 0;
    }

    pub fn active_alphabet(&self) -> ShapeVectorAlphabetId {
        self.active_alphabet
    }

    pub fn active_signature(&self) -> u64 {
        self.active_signature
    }

    pub fn rebuild_from_alphabet(
        &mut self,
        active_alphabet: ShapeVectorAlphabetId,
        alphabet: &ShapeVectorAlphabetData,
    ) {
        *self = Self::new_with_signature(
            active_alphabet,
            alphabet_signature(active_alphabet),
            &alphabet.entries,
        );
    }

    pub fn rebuild_from_entries(
        &mut self,
        active_alphabet: ShapeVectorAlphabetId,
        active_signature: u64,
        entries: &[CharacterEntry],
    ) {
        *self = Self::new_with_signature(active_alphabet, active_signature, entries);
    }

    pub fn rebuild_from_custom_alphabet(
        &mut self,
        active_alphabet: ShapeVectorAlphabetId,
        active_signature: u64,
        alphabet: &ShapeVectorAlphabetData,
    ) {
        self.rebuild_from_entries(active_alphabet, active_signature, &alphabet.entries);
    }
}

pub fn shape_vector_tuning_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<ShapeVectorConfig>,
) {
    let mut changed = false;

    if keys.just_pressed(KeyCode::BracketLeft) {
        config.distance_threshold = (config.distance_threshold - 0.005).max(0.0);
        changed = true;
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        config.distance_threshold = (config.distance_threshold + 0.005).min(1.0);
        changed = true;
    }
    if keys.just_pressed(KeyCode::Semicolon) {
        config.global_crunch_exponent = (config.global_crunch_exponent - 0.25).max(0.25);
        changed = true;
    }
    if keys.just_pressed(KeyCode::Quote) {
        config.global_crunch_exponent = (config.global_crunch_exponent + 0.25).min(16.0);
        changed = true;
    }
    if keys.just_pressed(KeyCode::Comma) {
        config.directional_crunch_exponent = (config.directional_crunch_exponent - 0.5).max(0.5);
        changed = true;
    }
    if keys.just_pressed(KeyCode::Period) {
        config.directional_crunch_exponent = (config.directional_crunch_exponent + 0.5).min(20.0);
        changed = true;
    }
    if keys.just_pressed(KeyCode::Minus) {
        config.sampling_quality = config.sampling_quality.saturating_sub(1).max(1);
        changed = true;
    }
    if keys.just_pressed(KeyCode::Equal) {
        config.sampling_quality = (config.sampling_quality + 1).min(32);
        changed = true;
    }
    if keys.just_pressed(KeyCode::F7) {
        config.enable_global_crunch = !config.enable_global_crunch;
        changed = true;
    }
    if keys.just_pressed(KeyCode::F8) {
        config.enable_directional_crunch = !config.enable_directional_crunch;
        changed = true;
    }
    if keys.just_pressed(KeyCode::F6) {
        config.alphabet = config.alphabet.next();
        changed = true;
    }
    if keys.just_pressed(KeyCode::F12) {
        config.mode = config.mode.next();
        changed = true;
    }
    if keys.just_pressed(KeyCode::Digit9) {
        config.structural_fallback_distance_threshold =
            (config.structural_fallback_distance_threshold - 0.01).max(0.0);
        changed = true;
    }
    if keys.just_pressed(KeyCode::Digit7) {
        config.contrast_adaptive_threshold_boost =
            (config.contrast_adaptive_threshold_boost - 0.05).max(0.0);
        changed = true;
    }
    if keys.just_pressed(KeyCode::Digit8) {
        config.contrast_adaptive_threshold_boost =
            (config.contrast_adaptive_threshold_boost + 0.05).min(4.0);
        changed = true;
    }
    if keys.just_pressed(KeyCode::Digit0) {
        config.structural_fallback_distance_threshold =
            (config.structural_fallback_distance_threshold + 0.01).min(2.5);
        changed = true;
    }
    if keys.just_pressed(KeyCode::F10) {
        config.enable_structural_fallback = !config.enable_structural_fallback;
        changed = true;
    }
    if keys.just_pressed(KeyCode::F11) {
        config.enable_contrast_adaptive_threshold = !config.enable_contrast_adaptive_threshold;
        changed = true;
    }
    if keys.just_pressed(KeyCode::Backslash) {
        *config = ShapeVectorConfig::default();
        changed = true;
    }

    if changed {
        info!(
            "Shape-vector tuning: mode={} alphabet={} threshold={:.3} adaptive={} boost={:.2} fallback={:.3} contrast={} global={:.2} directional={:.2} quality={} global_on={} directional_on={} structural_fb={} | keys: F12 mode F6 alphabet [] threshold 7/8 adaptive-boost 9/0 fallback ;' global ,./ directional -= quality F7/F8 toggles F10 structural F11 adaptive \\ reset",
            config.mode.as_str(),
            config.alphabet.as_str(),
            config.distance_threshold,
            config.enable_contrast_adaptive_threshold,
            config.contrast_adaptive_threshold_boost,
            config.structural_fallback_distance_threshold,
            config.structural_fallback_contrast_threshold,
            config.global_crunch_exponent,
            config.directional_crunch_exponent,
            config.sampling_quality,
            config.enable_global_crunch,
            config.enable_directional_crunch,
            config.enable_structural_fallback,
        );
    }
}

pub fn sample_to_lightness(
    sample: &crate::render::sample_buffer::Sample,
    materials: &[Material],
) -> f32 {
    if sample.spare & spare_bits::MESH_FLAG != 0 {
        let r5 = (sample.visual & 0x1F) as f32;
        let g5 = ((sample.visual >> 5) & 0x1F) as f32;
        let b5 = ((sample.visual >> 10) & 0x1F) as f32;
        let diffuse_scale = sample.diffuse as f32 / 255.0;
        let r = (r5 * 255.0 / 31.0) * diffuse_scale;
        let g = (g5 * 255.0 / 31.0) * diffuse_scale;
        let b = (b5 * 255.0 / 31.0) * diffuse_scale;
        (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255.0
    } else {
        let mat_idx = (sample.visual & 0x00FF) as usize;
        let Some(material) = materials.get(mat_idx) else {
            return 0.0;
        };
        let mat_cell = material.lookup(0, sample.diffuse);
        let r = mat_cell.bg[0] as f32;
        let g = mat_cell.bg[1] as f32;
        let b = mat_cell.bg[2] as f32;
        (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255.0
    }
}

pub fn sample_to_rgb(
    sample: &crate::render::sample_buffer::Sample,
    materials: &[Material],
) -> [f32; 3] {
    let diffuse_divisor = if (sample.spare & spare_bits::PARITY_MASK) == spare_bits::REFLECTION
        && !sample.is_mesh()
    {
        400.0
    } else {
        255.0
    };

    if sample.spare & spare_bits::MESH_FLAG != 0 {
        let (r8, g8, b8) = rgb555_to_rgb888(sample.visual);
        let diffuse_scale = sample.diffuse as f32 / diffuse_divisor;
        return [
            r8 as f32 * diffuse_scale,
            g8 as f32 * diffuse_scale,
            b8 as f32 * diffuse_scale,
        ];
    }

    let mat_idx = (sample.visual & 0x00FF) as usize;
    let Some(material) = materials.get(mat_idx) else {
        return [0.0, 0.0, 0.0];
    };
    let mat_cell = material.lookup(0, sample.diffuse);
    [
        mat_cell.bg[0] as f32,
        mat_cell.bg[1] as f32,
        mat_cell.bg[2] as f32,
    ]
}

fn bilinear_sample_rgb(
    buffer: &SampleBuffer,
    materials: &[Material],
    px: f32,
    py: f32,
) -> [f32; 3] {
    let px = px.clamp(0.0, (buffer.width - 1) as f32);
    let py = py.clamp(0.0, (buffer.height - 1) as f32);

    let x0 = px.floor() as u32;
    let y0 = py.floor() as u32;
    let x1 = (x0 + 1).min(buffer.width - 1);
    let y1 = (y0 + 1).min(buffer.height - 1);
    let fx = px - px.floor();
    let fy = py - py.floor();

    let c00 = sample_to_rgb(buffer.sample_at(x0, y0), materials);
    let c10 = sample_to_rgb(buffer.sample_at(x1, y0), materials);
    let c01 = sample_to_rgb(buffer.sample_at(x0, y1), materials);
    let c11 = sample_to_rgb(buffer.sample_at(x1, y1), materials);

    let lerp = |a: [f32; 3], b: [f32; 3], t: f32| -> [f32; 3] {
        [
            a[0] * (1.0 - t) + b[0] * t,
            a[1] * (1.0 - t) + b[1] * t,
            a[2] * (1.0 - t) + b[2] * t,
        ]
    };

    let top = lerp(c00, c10, fx);
    let bottom = lerp(c01, c11, fx);
    lerp(top, bottom, fy)
}

fn glyph_ink(pixel: image::Rgba<u8>) -> f32 {
    let r = pixel[0] as f32 / 255.0;
    let g = pixel[1] as f32 / 255.0;
    let b = pixel[2] as f32 / 255.0;
    let a = pixel[3] as f32 / 255.0;
    let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    if a > 0.0 { lum * a } else { lum }
}

pub fn optimize_glyph_colors(
    buffer: &SampleBuffer,
    materials: &[Material],
    cell_x: usize,
    cell_y: usize,
    glyph: u8,
) -> Option<([u8; 3], [u8; 3])> {
    if glyph == b' ' {
        return None;
    }

    let image = runtime_font_image();
    let glyph_x = (glyph as u32 % 16) * RUNTIME_FONT_CHAR_WIDTH;
    let glyph_y = (glyph as u32 / 16) * RUNTIME_FONT_CHAR_HEIGHT;
    let base_x = (2 + 2 * cell_x) as f32;
    let base_y = (2 + 2 * cell_y) as f32;

    let mut aa = 0.0f32;
    let mut ab = 0.0f32;
    let mut bb = 0.0f32;
    let mut rhs_a = [0.0f32; 3];
    let mut rhs_b = [0.0f32; 3];

    for gy in 0..RUNTIME_FONT_CHAR_HEIGHT {
        for gx in 0..RUNTIME_FONT_CHAR_WIDTH {
            let ink = glyph_ink(*image.get_pixel(glyph_x + gx, glyph_y + gy)).clamp(0.0, 1.0);
            let bg = 1.0 - ink;
            let sample_x = base_x + ((gx as f32 + 0.5) / RUNTIME_FONT_CHAR_WIDTH as f32) * 2.0;
            let sample_y = base_y + ((gy as f32 + 0.5) / RUNTIME_FONT_CHAR_HEIGHT as f32) * 2.0;
            let src = bilinear_sample_rgb(buffer, materials, sample_x, sample_y);

            aa += ink * ink;
            ab += ink * bg;
            bb += bg * bg;

            for channel in 0..3 {
                rhs_a[channel] += ink * src[channel];
                rhs_b[channel] += bg * src[channel];
            }
        }
    }

    let det = aa * bb - ab * ab;
    if det.abs() < 1e-6 {
        return None;
    }

    let mut fg = [0u8; 3];
    let mut bk = [0u8; 3];
    for channel in 0..3 {
        let fg_value = (rhs_a[channel] * bb - rhs_b[channel] * ab) / det;
        let bk_value = (aa * rhs_b[channel] - ab * rhs_a[channel]) / det;
        fg[channel] = fg_value.clamp(0.0, 255.0).round() as u8;
        bk[channel] = bk_value.clamp(0.0, 255.0).round() as u8;
    }

    Some((fg, bk))
}

fn bilinear_sample_lightness(
    buffer: &SampleBuffer,
    materials: &[Material],
    px: f32,
    py: f32,
) -> f32 {
    let px = px.clamp(0.0, (buffer.width - 1) as f32);
    let py = py.clamp(0.0, (buffer.height - 1) as f32);

    let x0 = px.floor() as u32;
    let y0 = py.floor() as u32;
    let x1 = (x0 + 1).min(buffer.width - 1);
    let y1 = (y0 + 1).min(buffer.height - 1);
    let fx = px - px.floor();
    let fy = py - py.floor();

    let l00 = sample_to_lightness(buffer.sample_at(x0, y0), materials);
    let l10 = sample_to_lightness(buffer.sample_at(x1, y0), materials);
    let l01 = sample_to_lightness(buffer.sample_at(x0, y1), materials);
    let l11 = sample_to_lightness(buffer.sample_at(x1, y1), materials);

    (l00 * (1.0 - fx) * (1.0 - fy) + l10 * fx * (1.0 - fy) + l01 * (1.0 - fx) * fy + l11 * fx * fy)
        .clamp(0.0, 1.0)
}

fn sample_circular_region(
    buffer: &SampleBuffer,
    materials: &[Material],
    base_x: f32,
    base_y: f32,
    point: SamplingPoint,
    circle_radius: f32,
    quality: usize,
) -> f32 {
    let quality = quality.max(1);
    let center_x = base_x + point.x * 2.0;
    let center_y = base_y + point.y * 2.0;

    if quality == 1 {
        return bilinear_sample_lightness(buffer, materials, center_x, center_y);
    }

    let radius = circle_radius * 2.0;
    let mut total = 0.0f32;
    for i in 0..quality {
        let radial = (((i as f32) + 0.5) / quality as f32).sqrt() * radius;
        let angle = GOLDEN_ANGLE * i as f32;
        let px = center_x + radial * angle.cos();
        let py = center_y + radial * angle.sin();
        total += bilinear_sample_lightness(buffer, materials, px, py);
    }

    total / quality as f32
}

fn sample_vector_with_points(
    buffer: &SampleBuffer,
    materials: &[Material],
    cell_x: usize,
    cell_y: usize,
    points: &[SamplingPoint],
    circle_radius: f32,
    quality: usize,
) -> Vec<f32> {
    let base_x = (2 + 2 * cell_x) as f32;
    let base_y = (2 + 2 * cell_y) as f32;
    points
        .iter()
        .copied()
        .map(|point| {
            sample_circular_region(
                buffer,
                materials,
                base_x,
                base_y,
                point,
                circle_radius,
                quality,
            )
        })
        .collect()
}

pub fn sample_cell_vector(
    buffer: &SampleBuffer,
    materials: &[Material],
    cell_x: usize,
    cell_y: usize,
) -> [f32; 6] {
    let alphabet = default_alphabet();
    sample_vector_with_points(
        buffer,
        materials,
        cell_x,
        cell_y,
        &alphabet.sampling_points,
        alphabet.circle_radius,
        DEFAULT_SAMPLING_QUALITY,
    )
    .try_into()
    .expect("internal sampling must produce 6 values")
}

pub struct ShapeVectorGlyphSelector<'a> {
    pub alphabet: &'a ShapeVectorAlphabetData,
    pub matcher: &'a mut ShapeVectorMatcher,
    pub materials: &'a [Material],
    pub water_z: f32,
    pub distance_threshold: f32,
    pub global_crunch_exponent: f32,
    pub directional_crunch_exponent: f32,
    pub sampling_quality: usize,
    pub enable_global_crunch: bool,
    pub enable_directional_crunch: bool,
    pub contrast_adaptive_threshold_boost: f32,
    pub enable_contrast_adaptive_threshold: bool,
}

impl GlyphSelector for ShapeVectorGlyphSelector<'_> {
    fn select_glyph(
        &mut self,
        sample_buffer: &SampleBuffer,
        cell_x: usize,
        cell_y: usize,
    ) -> Option<u8> {
        self.select_glyph_with_debug(sample_buffer, cell_x, cell_y)
            .glyph
    }
}

impl ShapeVectorGlyphSelector<'_> {
    pub fn select_glyph_with_debug(
        &mut self,
        sample_buffer: &SampleBuffer,
        cell_x: usize,
        cell_y: usize,
    ) -> ShapeVectorDecision {
        let sx = (2 + 2 * cell_x) as u32;
        let sy = (2 + 2 * cell_y) as u32;

        let heights = [
            sample_buffer.sample_at(sx, sy).height,
            sample_buffer.sample_at(sx + 1, sy).height,
            sample_buffer.sample_at(sx, sy + 1).height,
            sample_buffer.sample_at(sx + 1, sy + 1).height,
        ];
        if heights
            .iter()
            .all(|&height| height <= crate::render::sample_buffer::Sample::CLEAR_HEIGHT)
        {
            return ShapeVectorDecision::skipped(ShapeVectorSkipReason::Clear, None);
        }

        if self.water_z > f32::NEG_INFINITY
            && heights.iter().copied().fold(f32::NEG_INFINITY, f32::max) < self.water_z
        {
            return ShapeVectorDecision::skipped(ShapeVectorSkipReason::Underwater, None);
        }

        let mut vector: [f32; 6] = sample_vector_with_points(
            sample_buffer,
            self.materials,
            cell_x,
            cell_y,
            &self.alphabet.sampling_points,
            self.alphabet.circle_radius,
            self.sampling_quality,
        )
        .try_into()
        .expect("internal sampling must produce 6 values");

        if self.enable_directional_crunch && !self.alphabet.external_points.is_empty() {
            let external_vector = sample_vector_with_points(
                sample_buffer,
                self.materials,
                cell_x,
                cell_y,
                &self.alphabet.external_points,
                self.alphabet.circle_radius,
                self.sampling_quality,
            );
            directional_crunch_vector(
                &mut vector,
                &external_vector,
                &self.alphabet.affects_mapping,
                self.directional_crunch_exponent,
            );
        }

        if self.enable_global_crunch {
            crunch_vector(&mut vector, self.global_crunch_exponent);
        }

        let adaptive_contrast = vector_contrast(&vector);
        let effective_threshold = effective_distance_threshold(
            self.distance_threshold,
            adaptive_contrast,
            self.contrast_adaptive_threshold_boost,
            self.enable_contrast_adaptive_threshold,
        );
        let (glyph, distance) = self.matcher.find_glyph_with_distance(vector);
        if distance > effective_threshold {
            return ShapeVectorDecision::threshold_rejected(glyph, distance);
        }
        ShapeVectorDecision::accepted(glyph, distance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::material::test_materials;
    use crate::render::sample_buffer::{Sample, spare_bits as sb};

    #[test]
    fn test_default_alphabet_metadata_loaded() {
        let alphabet = default_alphabet();
        assert_eq!(alphabet.entries.len(), 95);
        assert_eq!(alphabet.sampling_points.len(), 6);
        assert_eq!(alphabet.external_points.len(), 10);
        assert_eq!(alphabet.affects_mapping.len(), 6);
        assert!((alphabet.circle_radius - 0.28125).abs() < 1e-6);
    }

    #[test]
    fn test_quantize_to_key_deterministic() {
        let v = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6];
        assert_eq!(quantize_to_key(&v), quantize_to_key(&v));
    }

    #[test]
    fn test_quantize_to_key_distinct() {
        let v1 = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6];
        let v2 = [0.6, 0.5, 0.4, 0.3, 0.2, 0.1];
        assert_ne!(quantize_to_key(&v1), quantize_to_key(&v2));
    }

    #[test]
    fn test_effective_distance_threshold_uses_contrast_boost() {
        let threshold = effective_distance_threshold(0.08, 0.8, 0.25, true);
        assert!((threshold - 0.28).abs() < 1e-6);
    }

    #[test]
    fn test_effective_distance_threshold_can_be_disabled() {
        let threshold = effective_distance_threshold(0.08, 0.8, 0.25, false);
        assert!((threshold - 0.08).abs() < 1e-6);
    }

    #[test]
    fn test_find_glyph_returns_valid_cp437() {
        let mut matcher = ShapeVectorMatcher::new_default();
        let glyph = matcher.find_glyph([0.1, 0.1, 0.3, 0.3, 0.05, 0.05]);
        assert!((0x20..=0x7E).contains(&glyph));
    }

    #[test]
    fn test_cache_hit() {
        let mut matcher = ShapeVectorMatcher::new_default();
        let v = [0.1, 0.2, 0.3, 0.4, 0.05, 0.05];
        let g1 = matcher.find_glyph(v);
        let g2 = matcher.find_glyph(v);
        assert_eq!(g1, g2);
        assert_eq!(matcher.cache_hits, 1);
        assert_eq!(matcher.cache_misses, 1);
    }

    #[test]
    fn test_cache_bounded() {
        let mut matcher = ShapeVectorMatcher::new_default();
        for i in 0..8193u32 {
            let v = [
                (i % 32) as f32 / 32.0,
                ((i / 32) % 32) as f32 / 32.0,
                ((i / 1024) % 32) as f32 / 32.0,
                0.0,
                0.0,
                0.0,
            ];
            matcher.find_glyph(v);
        }
        assert_eq!(matcher.cache.len(), 8192);
    }

    #[test]
    fn test_find_glyph_with_distance_returns_zero_for_exact_match() {
        let entries = [CharacterEntry {
            glyph: b'X',
            vector: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6],
        }];
        let mut matcher = ShapeVectorMatcher::new(ShapeVectorAlphabetId::Default, &entries);
        let (glyph, dist) = matcher.find_glyph_with_distance([0.1, 0.2, 0.3, 0.4, 0.5, 0.6]);
        assert_eq!(glyph, b'X');
        assert!(dist < 1e-10);
    }

    #[test]
    fn test_sample_to_lightness_range() {
        let materials = test_materials();
        let sample = Sample {
            visual: 31,
            diffuse: 255,
            spare: sb::MESH_FLAG,
            height: 10.0,
        };
        let l = sample_to_lightness(&sample, &materials);
        assert!((l - 0.2126).abs() < 0.01);
    }

    #[test]
    fn test_crunch_vector_preserves_max() {
        let mut v = [0.1, 0.3, 0.2, 0.4, 0.15, 0.05];
        let max_before = v.iter().copied().fold(0.0f32, f32::max);
        crunch_vector(&mut v, 3.0);
        let max_after = v.iter().copied().fold(0.0f32, f32::max);
        assert!((max_before - max_after).abs() < 1e-6);
    }

    #[test]
    fn test_directional_crunch_uses_external_context() {
        let mut v = [0.2, 0.2, 0.2, 0.2, 0.2, 0.2];
        let external = vec![0.8; 10];
        let mapping = default_alphabet().affects_mapping.clone();
        directional_crunch_vector(&mut v, &external, &mapping, 7.0);
        assert!(v.iter().all(|&c| c < 0.2));
    }

    #[test]
    fn test_sample_cell_vector_returns_six_values() {
        let mut buf = SampleBuffer::new(4, 4);
        let materials = test_materials();
        for dy in 0..3u32 {
            for dx in 0..3u32 {
                *buf.sample_at_mut(2 + dx, 2 + dy) = Sample {
                    visual: 31,
                    diffuse: 255,
                    spare: sb::MESH_FLAG,
                    height: 50.0,
                };
            }
        }
        let v = sample_cell_vector(&buf, &materials, 0, 0);
        assert_eq!(v.len(), 6);
        assert!(v.iter().all(|&c| c >= 0.0 && c <= 1.0));
    }

    #[test]
    fn test_glyph_selector_returns_none_for_clear_cells() {
        let buf = SampleBuffer::new(4, 4);
        let materials = test_materials();
        let mut matcher = ShapeVectorMatcher::new_default();
        let mut selector = ShapeVectorGlyphSelector {
            alphabet: default_alphabet(),
            matcher: &mut matcher,
            materials: &materials,
            water_z: f32::NEG_INFINITY,
            distance_threshold: 0.05,
            global_crunch_exponent: 3.0,
            directional_crunch_exponent: 7.0,
            sampling_quality: 8,
            enable_global_crunch: true,
            enable_directional_crunch: true,
            contrast_adaptive_threshold_boost: 0.25,
            enable_contrast_adaptive_threshold: true,
        };
        assert_eq!(selector.select_glyph(&buf, 0, 0), None);
        let decision = selector.select_glyph_with_debug(&buf, 0, 0);
        assert_eq!(decision.skip_reason, Some(ShapeVectorSkipReason::Clear));
    }

    #[test]
    fn test_glyph_selector_returns_some_for_mesh_cells() {
        let mut buf = SampleBuffer::new(4, 4);
        let materials = test_materials();
        for dy in 0..3u32 {
            for dx in 0..3u32 {
                *buf.sample_at_mut(2 + dx, 2 + dy) = Sample {
                    visual: 31 | (15 << 5) | (10 << 10),
                    diffuse: 200,
                    spare: sb::MESH_FLAG,
                    height: 50.0,
                };
            }
        }

        let mut matcher = ShapeVectorMatcher::new_default();
        let mut selector = ShapeVectorGlyphSelector {
            alphabet: default_alphabet(),
            matcher: &mut matcher,
            materials: &materials,
            water_z: f32::NEG_INFINITY,
            distance_threshold: 1.0,
            global_crunch_exponent: 3.0,
            directional_crunch_exponent: 7.0,
            sampling_quality: 8,
            enable_global_crunch: true,
            enable_directional_crunch: true,
            contrast_adaptive_threshold_boost: 0.25,
            enable_contrast_adaptive_threshold: true,
        };
        assert!(selector.select_glyph(&buf, 0, 0).is_some());
    }

    #[test]
    fn test_glyph_selector_skips_underwater() {
        let mut buf = SampleBuffer::new(4, 4);
        let materials = test_materials();
        for dy in 0..3u32 {
            for dx in 0..3u32 {
                *buf.sample_at_mut(2 + dx, 2 + dy) = Sample {
                    visual: 31,
                    diffuse: 200,
                    spare: sb::MESH_FLAG,
                    height: 10.0,
                };
            }
        }

        let mut matcher = ShapeVectorMatcher::new_default();
        let mut selector = ShapeVectorGlyphSelector {
            alphabet: default_alphabet(),
            matcher: &mut matcher,
            materials: &materials,
            water_z: 50.0,
            distance_threshold: 1.0,
            global_crunch_exponent: 3.0,
            directional_crunch_exponent: 7.0,
            sampling_quality: 8,
            enable_global_crunch: true,
            enable_directional_crunch: true,
            contrast_adaptive_threshold_boost: 0.25,
            enable_contrast_adaptive_threshold: true,
        };
        assert_eq!(selector.select_glyph(&buf, 0, 0), None);
        let decision = selector.select_glyph_with_debug(&buf, 0, 0);
        assert_eq!(
            decision.skip_reason,
            Some(ShapeVectorSkipReason::Underwater)
        );
    }

    #[test]
    fn test_glyph_selector_classifies_threshold_skip() {
        let mut buf = SampleBuffer::new(4, 4);
        let materials = test_materials();
        for dy in 0..3u32 {
            for dx in 0..3u32 {
                *buf.sample_at_mut(2 + dx, 2 + dy) = Sample {
                    visual: 31 | (15 << 5) | (10 << 10),
                    diffuse: 200,
                    spare: sb::MESH_FLAG,
                    height: 50.0,
                };
            }
        }

        let mut matcher = ShapeVectorMatcher::new_default();
        let mut selector = ShapeVectorGlyphSelector {
            alphabet: default_alphabet(),
            matcher: &mut matcher,
            materials: &materials,
            water_z: f32::NEG_INFINITY,
            distance_threshold: 0.0,
            global_crunch_exponent: 3.0,
            directional_crunch_exponent: 7.0,
            sampling_quality: 8,
            enable_global_crunch: true,
            enable_directional_crunch: true,
            contrast_adaptive_threshold_boost: 0.25,
            enable_contrast_adaptive_threshold: true,
        };
        let decision = selector.select_glyph_with_debug(&buf, 0, 0);
        assert_eq!(
            decision.skip_reason,
            Some(ShapeVectorSkipReason::DistanceThreshold)
        );
        assert!(decision.distance.is_some());
    }

    #[test]
    fn test_shape_vector_frame_stats_counts_colored_space_fallback() {
        let mut stats = ShapeVectorFrameStats::default();
        stats.begin_frame(1);
        stats.note_selection(
            ShapeVectorDecision::resolve_fallback(),
            b' ',
            b' ',
            [153, 153, 255],
            [102, 102, 204],
        );
        assert_eq!(stats.resolve_fallback_cells, 1);
        assert_eq!(stats.fallback_space_cells, 1);
        assert_eq!(stats.final_space_cells, 1);
        assert_eq!(stats.colored_space_cells, 1);
    }

    #[test]
    fn test_registry_exposes_two_alphabets() {
        let registry = ShapeVectorAlphabetRegistry::default();
        assert_eq!(
            registry.get(ShapeVectorAlphabetId::Default).entries.len(),
            95
        );
        assert!(registry.get(ShapeVectorAlphabetId::Minimal).entries.len() < 95);
        assert!(registry.get(ShapeVectorAlphabetId::Minimal).entries.len() >= 8);
    }

    #[test]
    fn test_matcher_rebuild_switches_alphabet() {
        let registry = ShapeVectorAlphabetRegistry::default();
        let mut matcher = ShapeVectorMatcher::new_default();
        assert_eq!(matcher.active_alphabet(), ShapeVectorAlphabetId::Default);
        matcher.rebuild_from_alphabet(
            ShapeVectorAlphabetId::Minimal,
            registry.get(ShapeVectorAlphabetId::Minimal),
        );
        assert_eq!(matcher.active_alphabet(), ShapeVectorAlphabetId::Minimal);
        let allowed = registry
            .get(ShapeVectorAlphabetId::Minimal)
            .entries
            .iter()
            .map(|entry| entry.glyph)
            .collect::<BTreeSet<_>>();
        let glyph = matcher.find_glyph([0.1, 0.1, 0.3, 0.3, 0.05, 0.05]);
        assert!(allowed.contains(&glyph));
    }
}
