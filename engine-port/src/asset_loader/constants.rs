// ---- Sprite constants (from sprite_constants.h) ----

/// Minimum number of layers required in a valid .xp sprite file.
/// Layer 0 = colorkey/transparency, Layer 1 = height, Layer 2 = visual.
pub const SPRITE_MIN_LAYERS: usize = 3;

/// Palette index used for swoosh marker cells.
pub const SPRITE_SWOOSH_INDEX: u8 = 254;

/// Palette index used for transparent cells.
pub const SPRITE_TRANSPARENT_INDEX: u8 = 255;

/// Cyan RGB color used to mark swoosh cells on the last sprite layer.
pub const SPRITE_CYAN: (u8, u8, u8) = (0, 255, 255);

/// Magenta RGB color used by REXPaint for transparency.
pub const SPRITE_MAGENTA: (u8, u8, u8) = (255, 0, 255);

/// CP437 glyph: full block.
pub const SPRITE_GLYPH_FULL_BLOCK: u32 = 219;

/// CP437 glyph: lower half block.
pub const SPRITE_GLYPH_HALF_LOWER: u32 = 220;

/// CP437 glyph: left half block.
pub const SPRITE_GLYPH_HALF_LEFT: u32 = 221;

/// CP437 glyph: right half block.
pub const SPRITE_GLYPH_HALF_RIGHT: u32 = 222;

/// CP437 glyph: upper half block.
pub const SPRITE_GLYPH_HALF_UPPER: u32 = 223;

/// Quadrant bitmask: bottom two quadrants (lower half block).
pub const SPRITE_MASK_LOWER: u8 = 0x3;

/// Quadrant bitmask: left two quadrants (left half block).
pub const SPRITE_MASK_LEFT: u8 = 0x5;

/// Quadrant bitmask: right two quadrants (right half block).
pub const SPRITE_MASK_RIGHT: u8 = 0xA;

/// Quadrant bitmask: top two quadrants (upper half block).
pub const SPRITE_MASK_UPPER: u8 = 0xC;

/// Quadrant bitmask: all four quadrants (full block).
pub const SPRITE_MASK_FULL: u8 = 0xF;

/// RGB increment applied when lightening colors for swoosh effect.
pub const SPRITE_LIGHTEN_AMOUNT: u8 = 51;

/// Height value indicating undefined/unset height.
pub const SPRITE_HEIGHT_UNDEFINED: u8 = 0xFF;

// ---- Terrain / A3D constants ----

/// Number of height cells per terrain patch edge (5x5 vertices, 4x4 quads).
pub const HEIGHT_CELLS: usize = 4;

/// Number of height vertices per terrain patch edge (HEIGHT_CELLS + 1).
pub const HEIGHT_CELLS_PLUS_ONE: usize = HEIGHT_CELLS + 1;

/// Number of visual (material) cells per terrain patch edge.
pub const VISUAL_CELLS: usize = 8;

/// Height scaling factor for terrain vertex heights.
pub const HEIGHT_SCALE: u16 = 16;

/// Magic number for A3D file format: "AS3D" as little-endian u32.
pub const A3D_MAGIC: u32 = 0x4433_5341;

/// Expected header size for A3D files (16 bytes).
pub const A3D_HEADER_SIZE: u32 = 16;

/// Size of a single FilePatch record in bytes.
pub const FILE_PATCH_SIZE: usize = 188;

/// Total size of the material table in bytes (256 materials x 512 bytes each).
pub const MATERIAL_TABLE_SIZE: usize = 131_072;
