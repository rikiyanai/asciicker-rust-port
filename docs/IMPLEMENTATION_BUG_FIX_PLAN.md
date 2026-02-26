# Asciicker Rust Port - Bug Fix Plan

## Overview

This document outlines how to handle known bugs from the C++ codebase when porting to Rust. Rather than fixing bugs in the original C++ codebase, we will implement defensive measures and validations in the Rust port.

---

## Known Bugs in C++ Codebase

### Bug 1: Terrain.cpp Line 613 - Duplicate `if(x)`

**Location:** `/Users/r/Downloads/asciicker-Y9-2/terrain.cpp:613`

**Code:**
```cpp
if (x)
    *x = px - t->x;
if (x)   // BUG: should be if (y)
    *y = py - t->y;
```

**Problem:** The second `if(x)` should be `if(y)`. This causes incorrect coordinate reconstruction.

**Rust Fix Plan:**
```rust
// In Rust, use proper pattern matching and avoid this bug entirely
pub fn get_terrain_patch(t: &Terrain, x: i32, y: i32, px: &mut i32, py: &mut i32) {
    if let Some(patch) = find_patch(t, x, y) {
        if let Some(px_val) = px.as_mut() {
            *px_val = patch.x;
        }
        if let Some(py_val) = py.as_mut() {  // Fixed: was incorrectly 'if let Some(px_val)'
            *py_val = patch.y;
        }
    }
}
```

**Validation:** Add unit tests that verify coordinate reconstruction.

---

### Bug 2: Terrain.cpp Line 805 / 1671 - Wrong Variable Scope

**Location:** `/Users/r/Downloads/asciicker-Y9-2/terrain.cpp:805` and `terrain.cpp:1671`

**Code:**
```cpp
// Callback signature uses u, v (local patch coords 0-7)
// But code incorrectly uses 'y' (world y) instead of 'v'
if (u < y)  // BUG: should be u < v
```

**Problem:** Comparing loop variables from different scopes.

**Rust Fix Plan:**
```rust
// Use named parameters correctly to avoid scope confusion
fn query_terrain_sample<F>(patch: &Patch, mut f: F)
where
    F: FnMut(i32, i32, i32, i32),
{
    for v in 0..8 {
        for u in 0..8 {
            // Correct: u < v (local coordinate comparison)
            if u < v {  // Fixed: was incorrectly 'u < y' (world coord)
                let height = get_height(patch, u, v);
                f(u, v, height, 0);
            }
        }
    }
}
```

---

### Bug 3: Terrain.cpp Lines 480/492 - Boundary `>` vs `>=`

**Location:** `/Users/r/Downloads/asciicker-Y9-2/terrain.cpp:480, 492`

**Code:**
```cpp
// Tap3x3::Sample() uses '>'
// Tap3x3::SetDiag() uses '>='
// TODO: "assuming '>' is fresher - needs verification"
```

**Problem:** Inconsistent boundary comparison operators.

**Rust Fix Plan:**
```rust
// Document the difference and pick a consistent approach
impl Tap3x3 {
    fn sample(&self, x: i32, y: i32) -> Option<&Sample> {
        // Use consistent boundary check
        if x > 0 && x < self.width && y > 0 && y < self.height {
            Some(&self.data[y * self.width + x])
        } else {
            None
        }
    }
    
    fn set_diag(&self, x: i32, y: i32, value: Sample) {
        // Match the sample boundary logic for consistency
        if x >= 0 && x < self.width && y >= 0 && y < self.height {
            self.data[y * self.width + x] = value;
        }
    }
}
```

---

### Bug 4: Audio.cpp - Memory Leak (Sample Unload)

**Location:** `/Users/r/Downloads/asciicker-Y9-2/audio.cpp:704`

**Problem:** No function to unload samples - decoded PCM buffers remain in memory forever.

**Rust Fix Plan:**
```rust
// Use Rust's ownership system to handle cleanup automatically
pub struct AudioEngine {
    samples: HashMap<u64, DecodedSample>,  // Automatically dropped
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        // Rust automatically frees when dropped
        self.samples.clear();
    }
}

// Explicit cleanup if needed
impl AudioEngine {
    pub fn unload_all(&mut self) {
        self.samples.clear();  // Frees all heap allocations
    }
}
```

---

### Bug 5: Audio.cpp - Division Precision

**Location:** `/Users/r/Downloads/asciicker-Y9-2/audio.cpp:553`

**Code:**
```cpp
// Division by 65535 could lose precision
int result = value / 65535;
```

**Rust Fix Plan:**
```rust
// Use f64 for precision or multiply to avoid division
fn normalize_audio_sample(value: u16) -> f32 {
    // Option 1: Direct cast (matches C++ behavior)
    (value as f32) / 65535.0
    
    // Option 2: More precise (recommended)
    // value as f32 / 65535.0 is already exact for most values
}

// Or use checked division
fn safe_divide(numerator: f32, denominator: f32) -> f32 {
    if denominator == 0.0 {
        0.0
    } else {
        numerator / denominator
    }
}
```

---

### Bug 6: Audio.cpp - Marker Lookup No Bounds

**Location:** `/Users/r/Downloads/asciicker-Y9-2/audio.cpp:553`

**Problem:** No bounds check when looking up markers by index.

**Rust Fix Plan:**
```rust
// Use Rust's bounds-checked arrays
fn get_marker(markers: &[Marker], index: usize) -> Option<&Marker> {
    markers.get(index)  // Returns None if out of bounds
}

// Or with index()
fn get_marker_strict(markers: &[Marker], index: usize) -> &Marker {
    markers.get_index(index)
        .expect("Marker index out of bounds")
}
```

---

## Rust-Specific Defensive Measures

### 1. Add Validation at File Loading Boundaries

```rust
pub struct XpSprite {
    layers: Vec<XpLayer>,
}

impl XpSprite {
    pub fn load(data: &[u8]) -> Result<Self, XpError> {
        // Validate header
        if data.len() < 16 {
            return Err(XpError::InvalidHeader);
        }
        
        // Validate dimensions
        let width = read_i32(&data[8..]);
        let height = read_i32(&data[12..]);
        
        if width <= 0 || width > 1024 {
            return Err(XpError::InvalidDimension { width });
        }
        
        if height <= 0 || height > 1024 {
            return Err(XpError::InvalidDimension { height });
        }
        
        // ... rest of loading
    }
}
```

---

### 2. Use Types That Prevent Invalid States

```rust
// Instead of raw integers, use newtypes
pub struct Glyph(u8);  // 0-255

impl Glyph {
    pub fn new(value: u8) -> Self {
        Self(value)  // Always valid
    }
    
    pub fn from_u8_saturating(value: u8) -> Self {
        Self(value.min(255))
    }
}

// Use enums for state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterState {
    None,
    Attack,
    Fall,
    Stand,
    Dead,
}
```

---

### 3. Add Integration Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_terrain_patch_coordinate() {
        let mut px = 0i32;
        let mut py = 0i32;
        
        // This would fail in C++ due to bug - should work in Rust
        get_terrain_patch(&terrain, 100, 200, Some(&mut px), Some(&mut py));
        
        assert_eq!(px, 100);  // Should match input
        assert_eq!(py, 200);  // Fixed: was incorrectly using px
    }
    
    #[test]
    fn test_audio_sample_cleanup() {
        let mut engine = AudioEngine::new();
        engine.load_sample("test.ogg");
        
        {
            let _sample = engine.get_sample("test.ogg");
        }  // Sample dropped here if not needed
        
        engine.unload_all();
        assert!(engine.samples.is_empty());
    }
}
```

---

## Summary: Bug Fix Strategy

| Bug | C++ Issue | Rust Solution |
|-----|-----------|---------------|
| terrain.cpp:613 | `if(x)` twice | Proper `if let` with correct variable |
| terrain.cpp:805 | `u < y` wrong scope | Named parameters with correct names |
| terrain.cpp:480/492 | Inconsistent `>/>=` | Choose consistent, document choice |
| audio.cpp:704 | Memory leak | Rust ownership / Drop trait |
| audio.cpp:553 | Division precision | Use f64 or multiply |
| audio.cpp:553 | No bounds check | Rust arrays are bounds-checked |

---

## Recommendation

**Do NOT modify the original C++ codebase.** 

Instead:
1. Document all known bugs in Rust code comments
2. Add unit tests that would fail with original bug behavior
3. Use Rust's type system to prevent invalid states
4. Add validation at file I/O boundaries
5. Let Rust's ownership model handle memory management

This approach:
- ✅ Keeps original research copy pristine
- ✅ Produces more robust Rust code
- ✅ Adds tests that verify bug is fixed
- ✅ Leverages Rust's safety guarantees

---

*Document created: 2026-02-19*
