use asciicker_engine::asset_loader::xp_sprite::{merge_layers, parse_xp, MergedCell, XpSprite};
use asciicker_engine::asset_loader::constants::{
    SPRITE_CYAN, SPRITE_GLYPH_HALF_LOWER, SPRITE_GLYPH_HALF_LEFT,
    SPRITE_GLYPH_HALF_RIGHT, SPRITE_GLYPH_HALF_UPPER, SPRITE_HEIGHT_UNDEFINED,
    SPRITE_LIGHTEN_AMOUNT,
};
use std::io::Write;

fn load_test_file(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/tests/golden/xp/{}",
        env!("CARGO_MANIFEST_DIR"),
        name
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read {}: {}", path, e))
}

/// Helper: create a synthetic gzip-compressed XP payload with the given layers.
/// Each layer is a vec of (glyph, fg, bg) tuples in column-major order.
fn make_xp_bytes(
    width: u32,
    height: u32,
    layers: &[Vec<(u32, [u8; 3], [u8; 3])>],
) -> Vec<u8> {
    let mut raw = Vec::new();

    // Global header: version, num_layers, width, height (all i32 LE)
    raw.extend_from_slice(&(-1i32).to_le_bytes());
    raw.extend_from_slice(&(layers.len() as i32).to_le_bytes());
    raw.extend_from_slice(&(width as i32).to_le_bytes());
    raw.extend_from_slice(&(height as i32).to_le_bytes());

    for (layer_idx, layer) in layers.iter().enumerate() {
        if layer_idx > 0 {
            // Per-layer width/height header (8 bytes)
            raw.extend_from_slice(&(width as i32).to_le_bytes());
            raw.extend_from_slice(&(height as i32).to_le_bytes());
        }
        for &(glyph, fg, bg) in layer {
            raw.extend_from_slice(&glyph.to_le_bytes());
            raw.extend_from_slice(&fg);
            raw.extend_from_slice(&bg);
        }
    }

    // Gzip compress
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&raw).unwrap();
    encoder.finish().unwrap()
}

// ---------- Test 1: parse item-apple.xp header ----------

#[test]
fn test_parse_item_apple_header() {
    let bytes = load_test_file("item-apple.xp");
    let sprite = parse_xp(&bytes).expect("Failed to parse item-apple.xp");

    assert!(sprite.width > 0, "width must be positive");
    assert!(sprite.height > 0, "height must be positive");
    assert!(
        sprite.layers.len() >= 3,
        "must have at least 3 layers, got {}",
        sprite.layers.len()
    );

    // Known values from binary inspection
    assert_eq!(sprite.width, 2);
    assert_eq!(sprite.height, 2);
    assert_eq!(sprite.layers.len(), 3);
}

// ---------- Test 2: parse grid-water.xp header ----------

#[test]
fn test_parse_grid_water_header() {
    let bytes = load_test_file("grid-water.xp");
    let sprite = parse_xp(&bytes).expect("Failed to parse grid-water.xp");

    assert!(sprite.width > 0, "width must be positive");
    assert!(sprite.height > 0, "height must be positive");
    assert!(
        sprite.layers.len() >= 3,
        "must have at least 3 layers, got {}",
        sprite.layers.len()
    );

    // Known values from binary inspection
    assert_eq!(sprite.width, 7);
    assert_eq!(sprite.height, 7);
    assert_eq!(sprite.layers.len(), 3);
}

// ---------- Test 3: column-major layout ----------

#[test]
fn test_column_major_layout() {
    let bytes = load_test_file("item-apple.xp");
    let sprite = parse_xp(&bytes).expect("Failed to parse item-apple.xp");

    let layer2 = &sprite.layers[2];
    assert_eq!(
        layer2.cells.len(),
        (sprite.width * sprite.height) as usize,
        "layer 2 cell count must equal width * height"
    );

    // Verify column-major: cell at index 0 is [col=0,row=0], index 1 is [col=0,row=1]
    // From binary inspection:
    // [0,0] glyph=44 (comma), [0,1] glyph=3 (heart), [1,0] glyph=6, [1,1] glyph=96
    assert_eq!(layer2.cells[0].glyph, 44, "cell [0,0] glyph");
    assert_eq!(layer2.cells[1].glyph, 3, "cell [0,1] glyph");
    assert_eq!(layer2.cells[2].glyph, 6, "cell [1,0] glyph");
    assert_eq!(layer2.cells[3].glyph, 96, "cell [1,1] glyph");
}

// ---------- Test 4: layer count minimum ----------

#[test]
fn test_layer_count_minimum() {
    let apple = parse_xp(&load_test_file("item-apple.xp")).unwrap();
    let water = parse_xp(&load_test_file("grid-water.xp")).unwrap();

    assert!(
        apple.layers.len() >= 3,
        "item-apple must have >= 3 layers"
    );
    assert!(
        water.layers.len() >= 3,
        "grid-water must have >= 3 layers"
    );
}

// ---------- Test 5: cell structure ----------

#[test]
fn test_cell_structure() {
    let bytes = load_test_file("item-apple.xp");
    let sprite = parse_xp(&bytes).unwrap();

    // Layer 2, cell [0,0]: glyph=44, fg=(85,255,85), bg=(0,170,0)
    let cell = &sprite.layers[2].cells[0];
    assert!(cell.glyph <= 255, "glyph must be in CP437 range (0-255)");
    assert_eq!(cell.glyph, 44);
    assert_eq!(cell.fg, [85, 255, 85]);
    assert_eq!(cell.bg, [0, 170, 0]);

    // Layer 0, cell [0,0]: glyph=49, fg=(0,0,0), bg=(0,170,0)
    let cell0 = &sprite.layers[0].cells[0];
    assert_eq!(cell0.glyph, 49);
    assert_eq!(cell0.fg, [0, 0, 0]);
    assert_eq!(cell0.bg, [0, 170, 0]);
}

// ---------- Test 6: invalid too few bytes ----------

#[test]
fn test_invalid_too_few_bytes() {
    // Fewer than 16 bytes after decompression should fail.
    // Create a gzip containing only 8 bytes.
    let mut encoder =
        flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&[0u8; 8]).unwrap();
    let compressed = encoder.finish().unwrap();

    let result = parse_xp(&compressed);
    assert!(result.is_err(), "should fail for too few bytes");
}

// ---------- Test 7: height layer glyph range ----------

#[test]
fn test_height_layer_glyph_range() {
    let bytes = load_test_file("grid-water.xp");
    let sprite = parse_xp(&bytes).unwrap();

    // Layer 1 is the height layer. Glyphs should be in the height-encoding
    // range: '0'-'9' (48-57), 'A'-'Z' (65-90), or map to SPRITE_HEIGHT_UNDEFINED.
    let layer1 = &sprite.layers[1];
    for cell in &layer1.cells {
        let glyph = cell.glyph;
        let is_digit = (48..=57).contains(&glyph); // '0'-'9'
        let is_upper = (65..=90).contains(&glyph); // 'A'-'Z'
        let is_other = true; // other values map to SPRITE_HEIGHT_UNDEFINED

        assert!(
            is_digit || is_upper || is_other,
            "height layer glyph {glyph} must be decodable"
        );
    }

    // Specifically check that glyph 48 ('0') appears (height = 0)
    let has_zero = layer1.cells.iter().any(|c| c.glyph == 48);
    assert!(has_zero, "grid-water height layer should contain glyph '0'");
}

// ---------- Test 8: swoosh merge last layer ----------

#[test]
fn test_swoosh_merge_last_layer() {
    // Build a synthetic 4-layer XP sprite (2x1) where layer 3 (last) has swoosh cells.
    let width: u32 = 2;
    let height: u32 = 1;

    // Layer 0: colorkey bg = (255, 0, 255) (magenta = transparent)
    let layer0 = vec![
        (0u32, [0, 0, 0], [255, 0, 255]), // [0,0] transparent
        (0u32, [0, 0, 0], [255, 0, 255]), // [1,0] transparent
    ];

    // Layer 1: height encoding ('1' = height 1)
    let layer1 = vec![
        (49u32, [0, 0, 0], [0, 0, 0]), // '1' = height 1
        (49u32, [0, 0, 0], [0, 0, 0]),
    ];

    // Layer 2: visual base with known colors
    let layer2 = vec![
        (65u32, [100, 100, 100], [50, 50, 50]), // 'A' with grey colors
        (66u32, [200, 200, 200], [80, 80, 80]),  // 'B' with lighter colors
    ];

    // Layer 3 (last): swoosh on cell [0,0], non-swoosh overwrite on cell [1,0]
    let layer3 = vec![
        // Swoosh cell: cyan fg (0,255,255) + half-block lower (220)
        (SPRITE_GLYPH_HALF_LOWER, [0, 255, 255], [10, 20, 30]),
        // Non-swoosh cell: normal overwrite (not cyan fg, not half-block)
        (88u32, [255, 128, 0], [30, 40, 50]),
    ];

    let xp_bytes = make_xp_bytes(width, height, &[layer0, layer1, layer2, layer3]);
    let sprite = parse_xp(&xp_bytes).expect("Failed to parse synthetic sprite");

    assert_eq!(sprite.layers.len(), 4);
    assert_eq!(sprite.width, 2);
    assert_eq!(sprite.height, 1);

    // Merge layers
    let merged = merge_layers(&sprite);
    assert_eq!(merged.len(), 2, "merged should have width*height cells");

    // Cell [0,0]: swoosh cell detected (cyan fg + half-block glyph on last layer).
    // The base cell from layer 2 should have its fg lightened by SPRITE_LIGHTEN_AMOUNT.
    // Base fg was [100, 100, 100], lightened = [151, 151, 151].
    let cell0 = &merged[0];
    let expected_fg_0 = 100u8.saturating_add(SPRITE_LIGHTEN_AMOUNT);
    assert_eq!(
        cell0.fg,
        [expected_fg_0, expected_fg_0, expected_fg_0],
        "swoosh cell should have lightened fg"
    );
    // Height should be decoded from layer 1 glyph '1' = 1
    assert_eq!(cell0.height, 1, "height from layer 1 glyph '1'");

    // Cell [1,0]: non-swoosh overwrite from last layer.
    // Should have the last layer's values directly.
    let cell1 = &merged[1];
    assert_eq!(cell1.glyph, 88, "non-swoosh cell should use last-layer glyph");
    assert_eq!(cell1.fg, [255, 128, 0], "non-swoosh cell should use last-layer fg");
    assert_eq!(cell1.bg, [30, 40, 50], "non-swoosh cell should use last-layer bg");
}
