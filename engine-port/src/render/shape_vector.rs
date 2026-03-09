//! 6D shape-vector glyph matching using k-d tree nearest-neighbor.
//!
//! Port of Alex Harri's CharacterMatcher / generateSamplingData system.
//! Uses the `six-samples` alphabet (95 printable ASCII characters, each
//! represented as a 6D vector of per-cell lightness samples).
//!
//! The `ShapeVectorMatcher` builds a k-d tree at startup and provides
//! `find_glyph_with_distance` for nearest-neighbor lookup with a quantized
//! LRU cache to avoid redundant tree traversals.
//!
//! The `ShapeVectorGlyphSelector` implements the `GlyphSelector` trait
//! from `resolve_bridge.rs`, allowing shape-vector glyph selection to be
//! injected into the render pipeline without modifying `resolve.rs`.

use std::num::NonZeroUsize;

use bevy::prelude::*;
use kiddo::float::distance::SquaredEuclidean;
use kiddo::KdTree;
use lru::LruCache;

use crate::render::material::Material;
use crate::render::resolve_bridge::GlyphSelector;
use crate::render::sample_buffer::{SampleBuffer, spare_bits};

// ---------------------------------------------------------------------------
// Sampling positions (from six-samples.json metadata)
// ---------------------------------------------------------------------------

/// 6 sampling positions within each character cell (normalized [0,1] coords).
/// 2-column x 3-row grid matching Alex Harri's six-samples alphabet.
/// These MUST match the positions used to generate the alphabet data.
pub const SAMPLING_POSITIONS: [(f32, f32); 6] = [
    (0.27, 0.18), (0.73, 0.18), // top row: left, right
    (0.27, 0.50), (0.73, 0.50), // middle row: left, right
    (0.27, 0.82), (0.73, 0.82), // bottom row: left, right
];

// ---------------------------------------------------------------------------
// CharacterEntry and alphabet data
// ---------------------------------------------------------------------------

/// A single entry in the shape-vector alphabet.
#[derive(Debug, Clone, Copy)]
pub struct CharacterEntry {
    /// CP437 glyph byte value.
    pub glyph: u8,
    /// 6D shape vector (lightness samples at 6 positions).
    pub vector: [f32; 6],
}

/// The six-samples alphabet: 95 printable ASCII characters (0x20-0x7E).
/// Data from Alex Harri's six-samples.json.
const ALPHABET: [CharacterEntry; 95] = [
    CharacterEntry { glyph: 32, vector: [0.0000000000, 0.0000000000, 0.0000000000, 0.0000000000, 0.0000000000, 0.0000000000] },
    CharacterEntry { glyph: 33, vector: [0.0168962752, 0.0097820541, 0.0662531194, 0.0473868093, 0.0037320504, 0.0013120490] },
    CharacterEntry { glyph: 34, vector: [0.1257526059, 0.1214811575, 0.0038787879, 0.0029518717, 0.0000000000, 0.0000000000] },
    CharacterEntry { glyph: 35, vector: [0.0602522050, 0.0765653473, 0.3232513369, 0.2949590018, 0.0456884613, 0.0196807347] },
    CharacterEntry { glyph: 36, vector: [0.1474597274, 0.1632772068, 0.1762852050, 0.2883137255, 0.0887819812, 0.0951089730] },
    CharacterEntry { glyph: 37, vector: [0.1591952766, 0.0857351119, 0.2814688057, 0.2644135472, 0.0505430425, 0.0894963190] },
    CharacterEntry { glyph: 38, vector: [0.1203003134, 0.0645090750, 0.3153654189, 0.2836791444, 0.0711713682, 0.0370726729] },
    CharacterEntry { glyph: 39, vector: [0.0374079743, 0.0300167651, 0.0000000000, 0.0000000000, 0.0000000000, 0.0000000000] },
    CharacterEntry { glyph: 42, vector: [0.0000000000, 0.0000000000, 0.2984812834, 0.2829233512, 0.0000000000, 0.0000000000] },
    CharacterEntry { glyph: 43, vector: [0.0000000000, 0.0000000000, 0.1816755793, 0.1643208556, 0.0000000000, 0.0000000000] },
    CharacterEntry { glyph: 44, vector: [0.0000000000, 0.0000000000, 0.0000000000, 0.0000000000, 0.1045848823, 0.0247831475] },
    CharacterEntry { glyph: 46, vector: [0.0000000000, 0.0000000000, 0.0000000000, 0.0000000000, 0.0094613310, 0.0050295211] },
    CharacterEntry { glyph: 47, vector: [0.0000000000, 0.1555944311, 0.1382816399, 0.0537183601, 0.1119615132, 0.0000000000] },
    CharacterEntry { glyph: 91, vector: [0.1753189008, 0.0782126977, 0.2003422460, 0.0000000000, 0.1295283913, 0.0771922152] },
    CharacterEntry { glyph: 93, vector: [0.0826590859, 0.1676069684, 0.0000000000, 0.1929696970, 0.0810263139, 0.1217289890] },
    CharacterEntry { glyph: 124, vector: [0.0494934033, 0.0363437568, 0.0632442068, 0.0474010695, 0.0246519426, 0.0165901305] },
    CharacterEntry { glyph: 40, vector: [0.0955171660, 0.0775858299, 0.2544884135, 0.0000000000, 0.0207157956, 0.0637218456] },
    CharacterEntry { glyph: 41, vector: [0.0832567971, 0.0826153510, 0.0000000000, 0.2486559715, 0.0686784751, 0.0141409724] },
    CharacterEntry { glyph: 58, vector: [0.0000000000, 0.0000000000, 0.0414117647, 0.0334973262, 0.0071433778, 0.0033530141] },
    CharacterEntry { glyph: 59, vector: [0.0000000000, 0.0000000000, 0.0414117647, 0.0334973262, 0.1059844012, 0.0234273635] },
    CharacterEntry { glyph: 60, vector: [0.0000000000, 0.0127997667, 0.2344099822, 0.0334973262, 0.0000000000, 0.0168379620] },
    CharacterEntry { glyph: 61, vector: [0.0000000000, 0.0000000000, 0.2081568627, 0.1993440285, 0.0000000000, 0.0000000000] },
    CharacterEntry { glyph: 62, vector: [0.0132371164, 0.0000000000, 0.0401711230, 0.2298181818, 0.0173482032, 0.0000000000] },
    CharacterEntry { glyph: 63, vector: [0.1155332021, 0.1153874189, 0.0779322638, 0.1349590018, 0.0147532619, 0.0000000000] },
    CharacterEntry { glyph: 64, vector: [0.1653035936, 0.1321962242, 0.2693903743, 0.3741033868, 0.1316714046, 0.1764122749] },
    CharacterEntry { glyph: 92, vector: [0.1570085283, 0.0000000000, 0.0661818182, 0.1224812834, 0.0000000000, 0.1108244041] },
    CharacterEntry { glyph: 94, vector: [0.1844303521, 0.1804504702, 0.0000000000, 0.0000000000, 0.0000000000, 0.0000000000] },
    CharacterEntry { glyph: 96, vector: [0.0666520883, 0.0514323201, 0.0000000000, 0.0000000000, 0.0000000000, 0.0000000000] },
    CharacterEntry { glyph: 123, vector: [0.0559516000, 0.0988993367, 0.1594581105, 0.0385026738, 0.0271448356, 0.0854872804] },
    CharacterEntry { glyph: 125, vector: [0.1089146439, 0.0452948466, 0.0487557932, 0.1492905526, 0.0898024637, 0.0189372403] },
    CharacterEntry { glyph: 126, vector: [0.0000000000, 0.0000000000, 0.1757290553, 0.1702531194, 0.0000000000, 0.0000000000] },
    CharacterEntry { glyph: 45, vector: [0.0000000000, 0.0000000000, 0.1282994652, 0.1239215686, 0.0000000000, 0.0000000000] },
    CharacterEntry { glyph: 95, vector: [0.0000000000, 0.0000000000, 0.0000000000, 0.0000000000, 0.1563379255, 0.1517749107] },
    CharacterEntry { glyph: 48, vector: [0.1125154895, 0.1042204242, 0.3766417112, 0.3198859180, 0.0403673737, 0.0347401414] },
    CharacterEntry { glyph: 49, vector: [0.0653400394, 0.0301042350, 0.0184385027, 0.1231229947, 0.0672206429, 0.0847146293] },
    CharacterEntry { glyph: 50, vector: [0.1346599606, 0.0872075224, 0.0675080214, 0.1809483066, 0.1004592171, 0.0691012464] },
    CharacterEntry { glyph: 51, vector: [0.1177491071, 0.0931846344, 0.0456042781, 0.2820962567, 0.0867701728, 0.0262409797] },
    CharacterEntry { glyph: 52, vector: [0.0198848313, 0.0038778337, 0.2747094474, 0.2356791444, 0.0000000000, 0.0378015890] },
    CharacterEntry { glyph: 53, vector: [0.1307967053, 0.0874553539, 0.1491622103, 0.2434367201, 0.0730519717, 0.0316641155] },
    CharacterEntry { glyph: 54, vector: [0.0889423427, 0.0789416138, 0.3336327986, 0.2531764706, 0.0404111087, 0.0427873752] },
    CharacterEntry { glyph: 55, vector: [0.1078650047, 0.1442087616, 0.0437504456, 0.1454260250, 0.0372476128, 0.0000000000] },
    CharacterEntry { glyph: 56, vector: [0.1166265763, 0.1157372986, 0.3243778966, 0.3193582888, 0.0609956994, 0.0535461768] },
    CharacterEntry { glyph: 57, vector: [0.1277935710, 0.1038851228, 0.2303885918, 0.3300819964, 0.0549019608, 0.0000000000] },
    CharacterEntry { glyph: 65, vector: [0.0363291785, 0.0295356804, 0.3014046346, 0.3053832442, 0.0543917195, 0.0578905168] },
    CharacterEntry { glyph: 66, vector: [0.1534076828, 0.1067716306, 0.3491622103, 0.3290695187, 0.0860995699, 0.0392594212] },
    CharacterEntry { glyph: 67, vector: [0.0888548728, 0.1374006852, 0.2890409982, 0.0000998217, 0.0179896494, 0.1040309060] },
    CharacterEntry { glyph: 68, vector: [0.1613382900, 0.0938115023, 0.2844491979, 0.3052834225, 0.0934762009, 0.0155842263] },
    CharacterEntry { glyph: 69, vector: [0.1328230921, 0.1106931992, 0.3239215686, 0.0963850267, 0.0677600408, 0.0906334281] },
    CharacterEntry { glyph: 70, vector: [0.1223267002, 0.1270938115, 0.3055115865, 0.1114295900, 0.0485603907, 0.0000000000] },
    CharacterEntry { glyph: 71, vector: [0.1082003061, 0.1243239303, 0.3037433155, 0.2685204991, 0.0438224360, 0.0860995699] },
    CharacterEntry { glyph: 72, vector: [0.0963627087, 0.0964647569, 0.3606702317, 0.3540962567, 0.0553101538, 0.0553538888] },
    CharacterEntry { glyph: 73, vector: [0.1043662074, 0.1001239157, 0.0746951872, 0.0607344029, 0.0780231795, 0.0737517312] },
    CharacterEntry { glyph: 74, vector: [0.0442160507, 0.1441212916, 0.0000000000, 0.2730409982, 0.0867118595, 0.0242729062] },
    CharacterEntry { glyph: 75, vector: [0.0956629492, 0.1011006633, 0.3714509804, 0.1337754011, 0.0547270209, 0.0689117283] },
    CharacterEntry { glyph: 76, vector: [0.0907208980, 0.0000000000, 0.2633155080, 0.0000000000, 0.0613601574, 0.0994095780] },
    CharacterEntry { glyph: 77, vector: [0.1479699687, 0.1469786428, 0.4216470588, 0.4136042781, 0.0519717181, 0.0534295503] },
    CharacterEntry { glyph: 78, vector: [0.1441212916, 0.0895254756, 0.3323208556, 0.3567771836, 0.0514323201, 0.0782710110] },
    CharacterEntry { glyph: 79, vector: [0.1237407974, 0.1197463372, 0.2962994652, 0.2941176471, 0.0484583424, 0.0423791822] },
    CharacterEntry { glyph: 80, vector: [0.1394853852, 0.1319775494, 0.3346595365, 0.2414688057, 0.0531379838, 0.0000000000] },
    CharacterEntry { glyph: 81, vector: [0.1236970625, 0.1195713973, 0.2938894831, 0.2906381462, 0.0490268970, 0.2111815730] },
    CharacterEntry { glyph: 82, vector: [0.1500255121, 0.1233034478, 0.3496898396, 0.3217540107, 0.0555725636, 0.0657628107] },
    CharacterEntry { glyph: 83, vector: [0.1358262264, 0.1158539252, 0.1799358289, 0.2533048128, 0.0881988483, 0.0521466579] },
    CharacterEntry { glyph: 84, vector: [0.1493840659, 0.1416721335, 0.0765775401, 0.0560285205, 0.0018368686, 0.0006851811] },
    CharacterEntry { glyph: 85, vector: [0.0956629492, 0.0973831912, 0.2829090909, 0.2862032086, 0.0514468985, 0.0446971354] },
    CharacterEntry { glyph: 86, vector: [0.1023106640, 0.0958670457, 0.2390873440, 0.2228021390, 0.0027698812, 0.0012828923] },
    CharacterEntry { glyph: 87, vector: [0.0959253590, 0.0885778847, 0.4182245989, 0.4021247772, 0.0745243822, 0.0696406444] },
    CharacterEntry { glyph: 88, vector: [0.1022231941, 0.0949486114, 0.2262245989, 0.2097967914, 0.0589401560, 0.0625993148] },
    CharacterEntry { glyph: 89, vector: [0.1054012683, 0.0989576500, 0.1806488414, 0.1629803922, 0.0017202420, 0.0009475909] },
    CharacterEntry { glyph: 90, vector: [0.1070486187, 0.1666010642, 0.1678573975, 0.0856755793, 0.1021940375, 0.0936365624] },
    CharacterEntry { glyph: 97, vector: [0.0000000000, 0.0000000000, 0.2250837790, 0.2924491979, 0.0817989649, 0.0576281070] },
    CharacterEntry { glyph: 98, vector: [0.1327939354, 0.0000000000, 0.2951016043, 0.2881568627, 0.0683140171, 0.0501785844] },
    CharacterEntry { glyph: 99, vector: [0.0000000000, 0.0000000000, 0.2690481283, 0.0547878788, 0.0277571252, 0.0741599242] },
    CharacterEntry { glyph: 100, vector: [0.0000000000, 0.1323128508, 0.2863315508, 0.2864598930, 0.0609519644, 0.0568554559] },
    CharacterEntry { glyph: 101, vector: [0.0000000000, 0.0000000000, 0.3347736185, 0.2466167558, 0.0377432757, 0.0627305197] },
    CharacterEntry { glyph: 102, vector: [0.0389532765, 0.1364385159, 0.2223030303, 0.0670374332, 0.0131788031, 0.0000000000] },
    CharacterEntry { glyph: 103, vector: [0.0000000000, 0.0080326554, 0.2854616756, 0.2267950089, 0.1918215613, 0.2613747358] },
    CharacterEntry { glyph: 104, vector: [0.1315839347, 0.0000000000, 0.2960855615, 0.2654973262, 0.0536336468, 0.0532837670] },
    CharacterEntry { glyph: 105, vector: [0.0471754501, 0.0413878563, 0.0608342246, 0.1084777184, 0.0661418471, 0.0824112545] },
    CharacterEntry { glyph: 106, vector: [0.0005831329, 0.0938406589, 0.0263672014, 0.2314866310, 0.1240032072, 0.1325169473] },
    CharacterEntry { glyph: 107, vector: [0.1326627305, 0.0000000000, 0.3520427807, 0.1865383244, 0.0534003936, 0.0671331730] },
    CharacterEntry { glyph: 108, vector: [0.1647641956, 0.0014578322, 0.1495187166, 0.0064171123, 0.0000000000, 0.0781252278] },
    CharacterEntry { glyph: 109, vector: [0.0000000000, 0.0000000000, 0.3458680927, 0.3270160428, 0.0507762956, 0.0503243677] },
    CharacterEntry { glyph: 110, vector: [0.0000000000, 0.0000000000, 0.2944741533, 0.2687344029, 0.0536336468, 0.0532837670] },
    CharacterEntry { glyph: 111, vector: [0.0000000000, 0.0000000000, 0.2753226381, 0.2723279857, 0.0471171368, 0.0405277353] },
    CharacterEntry { glyph: 112, vector: [0.0000000000, 0.0000000000, 0.2930766488, 0.2829376114, 0.2185436256, 0.0492455718] },
    CharacterEntry { glyph: 113, vector: [0.0000000000, 0.0000000000, 0.2803707665, 0.2849625668, 0.0620161819, 0.2074349442] },
    CharacterEntry { glyph: 114, vector: [0.0000000000, 0.0000000000, 0.2529910873, 0.1310516934, 0.0727458270, 0.0088927764] },
    CharacterEntry { glyph: 115, vector: [0.0000000000, 0.0000000000, 0.2163707665, 0.2084420677, 0.0811429404, 0.0445659305] },
    CharacterEntry { glyph: 116, vector: [0.0328303812, 0.0000000000, 0.2348377897, 0.0222745098, 0.0001166266, 0.0847875210] },
    CharacterEntry { glyph: 117, vector: [0.0000000000, 0.0000000000, 0.2709447415, 0.2687344029, 0.0670165464, 0.0534732852] },
    CharacterEntry { glyph: 118, vector: [0.0000000000, 0.0000000000, 0.2437076649, 0.2309162210, 0.0041985567, 0.0018368686] },
    CharacterEntry { glyph: 119, vector: [0.0000000000, 0.0000000000, 0.3799786096, 0.3740178253, 0.0714775129, 0.0709235367] },
    CharacterEntry { glyph: 120, vector: [0.0000000000, 0.0000000000, 0.2474866310, 0.2303743316, 0.0591150959, 0.0612726875] },
    CharacterEntry { glyph: 121, vector: [0.0000000000, 0.0000000000, 0.2474153298, 0.2339251337, 0.1434798455, 0.0169545885] },
    CharacterEntry { glyph: 122, vector: [0.0000000000, 0.0000000000, 0.1135401070, 0.1896042781, 0.0819884831, 0.0722501640] },
];

// ---------------------------------------------------------------------------
// Quantization
// ---------------------------------------------------------------------------

/// Quantize a 6D vector to a 30-bit cache key (5 bits per component).
///
/// Uses `((v * 32.0).floor() as u32).min(31)` per component and packs
/// into a single `u32` via 5-bit shifts.
///
/// P7-001 FIX: Corrected from original research's buggy 3-bit quantization.
/// R19-M3: Actual alphabet values only reach ~0.42, so levels 14-31 are
/// unused. This is acceptable -- unused key space does not affect correctness.
fn quantize_to_key(v: &[f32; 6]) -> u32 {
    let mut key: u32 = 0;
    for &c in v.iter() {
        let q = ((c * 32.0).floor() as u32).min(31);
        key = (key << 5) | q;
    }
    key
}

// ---------------------------------------------------------------------------
// Contrast crunch (R20-F04)
// ---------------------------------------------------------------------------

/// Apply global contrast crunch: normalize and exponentiate to exaggerate
/// the dominant direction of the 6D vector, making edges sharper.
/// Cost: ~0.01ms/frame (negligible).
fn crunch_vector(v: &mut [f32; 6], exponent: f32) {
    let max = v.iter().copied().fold(0.0f32, f32::max);
    if max < 1e-6 {
        return; // avoid div-by-zero for black cells
    }
    for c in v.iter_mut() {
        *c = (*c / max).powf(exponent) * max;
    }
}

// ---------------------------------------------------------------------------
// ShapeVectorMatcher
// ---------------------------------------------------------------------------

/// K-d tree backed nearest-neighbor matcher for 6D shape vectors.
///
/// Builds a k-d tree from the six-samples alphabet at startup and provides
/// cached glyph lookup via `find_glyph_with_distance`.
///
/// Cache: bounded LRU with 8192 entries (~130KB), mapping quantized 30-bit
/// keys to (glyph, distance) pairs. R61 FIX: prevents unbounded growth.
#[derive(Resource)]
pub struct ShapeVectorMatcher {
    tree: KdTree<f32, 6>,
    cache: LruCache<u32, (u8, f32)>,
    entries: Vec<CharacterEntry>,
    cache_hits: u64,
    cache_misses: u64,
}

impl ShapeVectorMatcher {
    /// Create a new matcher from the built-in alphabet data.
    pub fn new_default() -> Self {
        Self::new(&ALPHABET)
    }

    /// Create a new matcher from custom character entries.
    pub fn new(characters: &[CharacterEntry]) -> Self {
        let mut tree: KdTree<f32, 6> = KdTree::new();
        for (i, entry) in characters.iter().enumerate() {
            tree.add(&entry.vector, i as u64);
        }
        Self {
            tree,
            cache: LruCache::new(NonZeroUsize::new(8192).unwrap()),
            entries: characters.to_vec(),
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    /// Find the nearest glyph and return it with squared Euclidean distance.
    ///
    /// Uses LRU cache for repeated vectors. R20-F05: distance enables
    /// threshold-based fallback to auto_mat.
    ///
    /// `&mut self` required because `LruCache::get()` promotes to most-recent.
    pub fn find_glyph_with_distance(&mut self, vector: [f32; 6]) -> (u8, f32) {
        let key = quantize_to_key(&vector);
        if let Some(&(glyph, dist)) = self.cache.get(&key) {
            self.cache_hits += 1;
            return (glyph, dist);
        }
        self.cache_misses += 1;
        let nearest = self.tree.nearest_one::<SquaredEuclidean>(&vector);
        let glyph = self.entries[nearest.item as usize].glyph;
        let dist = nearest.distance;
        self.cache.put(key, (glyph, dist));
        (glyph, dist)
    }

    /// Find the nearest glyph (convenience wrapper, ignores distance).
    pub fn find_glyph(&mut self, vector: [f32; 6]) -> u8 {
        self.find_glyph_with_distance(vector).0
    }

    /// Cache hit rate for Pitfall 3 monitoring.
    /// R19-M1: 32-level quantization may produce lower hit rates than
    /// Alex Harri's 8-level. Monitor during profiling.
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            return 0.0;
        }
        self.cache_hits as f64 / total as f64
    }

    /// Clear the LRU cache and reset hit/miss counters.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        self.cache_hits = 0;
        self.cache_misses = 0;
    }
}

// ---------------------------------------------------------------------------
// Sampling from SampleBuffer
// ---------------------------------------------------------------------------

/// Extract lightness from a single sample, using dual-path for mesh vs terrain.
///
/// P7-002/P7-003 FIX: Mesh (MESH_FLAG set) uses RGB555 + diffuse scaling.
/// Terrain (MESH_FLAG clear) uses material shade table lookup.
/// R19-H1 FIX: Diffuse scaling applied before BT.709 for mesh samples.
/// R20-F06 FIX: Bounds check for invalid terrain material index.
pub fn sample_to_lightness(sample: &crate::render::sample_buffer::Sample, materials: &[Material]) -> f32 {
    if sample.spare & spare_bits::MESH_FLAG != 0 {
        // Mesh path: RGB555 -> RGB888, scale by diffuse, then BT.709
        let r5 = (sample.visual & 0x1F) as f32;
        let g5 = ((sample.visual >> 5) & 0x1F) as f32;
        let b5 = ((sample.visual >> 10) & 0x1F) as f32;
        let r8 = r5 * 255.0 / 31.0;
        let g8 = g5 * 255.0 / 31.0;
        let b8 = b5 * 255.0 / 31.0;
        // R19-H1: Apply diffuse scaling before BT.709
        let diffuse_scale = sample.diffuse as f32 / 255.0;
        let r = r8 * diffuse_scale;
        let g = g8 * diffuse_scale;
        let b = b8 * diffuse_scale;
        // BT.709 luminance
        (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255.0
    } else {
        // Terrain path: material shade table lookup
        let mat_idx = (sample.visual & 0x00FF) as usize;
        if mat_idx >= materials.len() {
            return 0.0; // R20-F06: invalid material -> black
        }
        let elevation = 0u8; // default elevation for lightness sampling
        let mat_cell = materials[mat_idx].lookup(elevation, sample.diffuse);
        // Use fg color from shade table (already incorporates diffuse)
        let r = mat_cell.fg[0] as f32;
        let g = mat_cell.fg[1] as f32;
        let b = mat_cell.fg[2] as f32;
        (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255.0
    }
}

/// Compute the 6D shape vector for a cell by bilinear interpolation at
/// 6 sampling positions within the 2x2 sample block.
///
/// P7-057 FIX (AUTHORITATIVE): bilinear interpolation, NOT circle-area averaging.
/// P7-204 FIX: cast intermediate coordinates to u32 for sample_at().
/// R20-F12: Y-axis orientation matches SampleBuffer convention (y-up).
pub fn sample_cell_vector(
    buffer: &SampleBuffer,
    materials: &[Material],
    cell_x: usize,
    cell_y: usize,
) -> [f32; 6] {
    // Base sample coordinates for this cell's 2x2 block
    let base_x = (2 + 2 * cell_x) as f32;
    let base_y = (2 + 2 * cell_y) as f32;

    let mut vector = [0.0f32; 6];

    for (i, &(sx, sy)) in SAMPLING_POSITIONS.iter().enumerate() {
        // Map normalized position [0,1] to sample-buffer sub-pixel position
        // within the 2x2 block (0..2 range)
        let px = base_x + sx * 2.0;
        let py = base_y + sy * 2.0;

        // Bilinear interpolation corners
        let x0 = px.floor() as u32;
        let y0 = py.floor() as u32;
        let x1 = (x0 + 1).min(buffer.width - 1);
        let y1 = (y0 + 1).min(buffer.height - 1);

        let fx = px - px.floor();
        let fy = py - py.floor();

        // Four corner lightness values
        let l00 = sample_to_lightness(buffer.sample_at(x0, y0), materials);
        let l10 = sample_to_lightness(buffer.sample_at(x1, y0), materials);
        let l01 = sample_to_lightness(buffer.sample_at(x0, y1), materials);
        let l11 = sample_to_lightness(buffer.sample_at(x1, y1), materials);

        // Bilinear interpolation
        let l = l00 * (1.0 - fx) * (1.0 - fy)
            + l10 * fx * (1.0 - fy)
            + l01 * (1.0 - fx) * fy
            + l11 * fx * fy;

        vector[i] = l.clamp(0.0, 1.0);
    }

    vector
}

// ---------------------------------------------------------------------------
// ShapeVectorGlyphSelector (GlyphSelector trait impl)
// ---------------------------------------------------------------------------

/// GlyphSelector implementation using 6D shape-vector k-d tree matching.
///
/// Created per-frame in render_pipeline_system from ShapeVectorMatcher resource.
/// R20-F03: water_z field for underwater cell skipping.
/// R20-F05: distance_threshold for auto_mat fallback.
pub struct ShapeVectorGlyphSelector<'a> {
    /// Mutable reference to the cached matcher.
    pub matcher: &'a mut ShapeVectorMatcher,
    /// Material library for terrain lightness extraction (P7-003).
    pub materials: &'a [Material],
    /// Water surface height (R20-F03: skip underwater cells).
    pub water_z: f32,
    /// Squared Euclidean distance threshold for fallback (R20-F05).
    pub distance_threshold: f32,
}

impl GlyphSelector for ShapeVectorGlyphSelector<'_> {
    fn select_glyph(
        &mut self,
        sample_buffer: &SampleBuffer,
        cell_x: usize,
        cell_y: usize,
    ) -> Option<u8> {
        let sx = (2 + 2 * cell_x) as u32;
        let sy = (2 + 2 * cell_y) as u32;

        // R20-F02: Guard against clear/sky cells
        if sample_buffer.sample_at(sx, sy).height <= crate::render::sample_buffer::Sample::CLEAR_HEIGHT
            && sample_buffer.sample_at(sx + 1, sy).height <= crate::render::sample_buffer::Sample::CLEAR_HEIGHT
            && sample_buffer.sample_at(sx, sy + 1).height <= crate::render::sample_buffer::Sample::CLEAR_HEIGHT
            && sample_buffer.sample_at(sx + 1, sy + 1).height <= crate::render::sample_buffer::Sample::CLEAR_HEIGHT
        {
            return None;
        }

        // R20-F03: Skip underwater cells to preserve ripple glyph
        if self.water_z > f32::NEG_INFINITY {
            let max_h = sample_buffer.sample_at(sx, sy).height
                .max(sample_buffer.sample_at(sx + 1, sy).height)
                .max(sample_buffer.sample_at(sx, sy + 1).height)
                .max(sample_buffer.sample_at(sx + 1, sy + 1).height);
            if max_h < self.water_z {
                return None;
            }
        }

        let mut vector = sample_cell_vector(sample_buffer, self.materials, cell_x, cell_y);

        // R20-F04: Global contrast crunch
        crunch_vector(&mut vector, 1.5);

        // R20-F05: Distance threshold fallback
        let (glyph, distance) = self.matcher.find_glyph_with_distance(vector);
        if distance > self.distance_threshold {
            return None; // Fall back to auto_mat
        }

        Some(glyph)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
