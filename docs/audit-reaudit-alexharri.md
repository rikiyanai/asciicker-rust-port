# Alex Harri ASCII Renderer - Audit & Re-Audit Findings

**Source:** `/Users/r/Projects/ascii research/alexharri-ascii/website_repo/website-master/`
**Generated:** 2026-02-19

---

## 1. 6D Vector Structure - Alphabet JSON/Config

### Location
`/Users/r/Projects/ascii research/alexharri-ascii/website_repo/website-master/src/components/AsciiScene/alphabets/`

### Files Found
- `default.json` - Main alphabet (95 characters, 6D vectors)
- `six-samples.json` - Alternative alphabet with 6 sampling points
- `two-samples.json` - Alternative alphabet with 2 sampling points  
- `pixel-short.json` - Single sample pixel alphabet
- `simple-directional-crunch.json` - Directional crunch variant

### default.json Structure
```json
{
  "metadata": {
    "samplingConfig": {
      "points": [...],        // 6 sampling circle centers (normalized 0-1)
      "externalPoints": [...], // 10 external sampling points
      "circleRadius": 0.28125
    },
    "fontSize": 1,
    "width": 1,
    "height": 1.3333333333333333
  },
  "characters": [
    { "char": " ", "vector": [0, 0, 0, 0, 0, 0] },
    { "char": "!", "vector": [0.01689627523872002, 0.00978205408557475, ...] },
    ...
  ]
}
```

### Key Details
- **Number of characters:** 95
- **Vector dimensions:** 6 (for default.json with 6 sampling points)
- **Index 0 character:** Space (" ")
- **Vector component range:** ~0.0 to ~0.36 (pre-normalized)

### Source Location
- Primary: `/Users/r/Projects/ascii research/alexharri-ascii/website_repo/website-master/src/components/AsciiScene/alphabets/default.json`
- Alphabet loading: `/Users/r/Projects/ascii research/alexharri-ascii/website_repo/website-master/src/components/AsciiScene/alphabets/index.ts`

---

## 2. Sampling Circle Radius

### Value
- **circleRadius:** `0.28125` (normalized, unitless)
- This is multiplied by the cell size (min of boxWidth, boxHeight) at runtime

### Usage in Code
```typescript
// From renderConfig.ts / AsciiRenderConfig
this.samplePointRadius = fontSize * metadata.samplingConfig.circleRadius;
```

### Source Locations
- **JSON config:** `/Users/r/Projects/ascii research/alexharri-ascii/website_repo/website-master/src/components/AsciiScene/alphabets/default.json` (line 1)
- **Usage:** `/Users/r/Projects/ascii research/alexharri-ascii/website_repo/website-master/src/components/AsciiScene/renderConfig.ts`
- **Shader:** `/Users/r/Projects/ascii research/alexharri-ascii/website_repo/website-master/src/components/AsciiScene/sampling/gpu/shaders.ts` (line 30: `uniform float u_circleRadius`)

---

## 3. External Point Positions

### Location in default.json
```json
"externalPoints": [
  {"x": 0.07, "y": -0.21, "affects": [0,1]},
  {"x": 0.93, "y": -0.21, "affects": [0,1]},
  {"x": -0.25, "y": 0.07, "affects": [0,2]},
  {"x": 1.25, "y": 0.07, "affects": [1,3]},
  {"x": -0.25, "y": 0.5, "affects": [0,2,4]},
  {"x": 1.25, "y": 0.5, "affects": [1,3,5]},
  {"x": -0.25, "y": 0.93, "affects": [2,4]},
  {"x": 1.25, "y": 0.93, "affects": [3,5]},
  {"x": 0.07, "y": 1.21, "affects": [4,5]},
  {"x": 0.93, "y": 1.21, "affects": [4,5]}
]
```

### Key Details
- **Count:** 10 external points
- **Position range:** x: -0.25 to 1.25, y: -0.21 to 1.21 (normalized, can extend outside cell)
- **affects:** Array of sampling point indices that this external point influences
- **affectsMapping:** Pre-computed mapping from each internal point to its influencing external points

### Source Location
- `/Users/r/Projects/ascii research/alexharri-ascii/website_repo/website-master/src/components/AsciiScene/alphabets/default.json`

---

## 4. Crunch Exponent Values

### Default Values
- **globalCrunchExponent:** `3` (default)
- **directionalCrunchExponent:** `7` (default)

### Source Code Reference
From `/Users/r/Projects/ascii research/alexharri-ascii/website_repo/website-master/src/components/AsciiScene/sampling/gpu/GPUSamplingDataGenerator.ts` (lines 115-116):
```typescript
this.globalCrunchExponent = options.globalCrunchExponent ?? 3;
this.directionalCrunchExponent = options.directionalCrunchExponent ?? 7;
```

### Shader Implementation

#### Directional Crunch (shaders.ts lines 262-288)
```glsl
uniform float u_directionalCrunchExponent;
// ...
if (contextValue > value) {
  float normalized = value / contextValue;
  float enhanced = pow(normalized, u_directionalCrunchExponent);
  value = enhanced * contextValue;
}
```

#### Global Crunch (shaders.ts lines 290-324)
```glsl
uniform float u_globalCrunchExponent;
// ...
if (maxValue > 0.0) {
  float normalized = value / maxValue;
  float enhanced = pow(normalized, u_globalCrunchExponent);
  value = enhanced * maxValue;
}
```

### Effect Description
- **Global Crunch:** Normalizes by per-cell maximum value, applies exponent power curve, then rescales. Enhances within-cell contrast.
- **Directional Crunch:** Uses external sampling points from neighboring cells to darken edges. Higher exponent = stronger edge effect.

### Source Locations
- **Defaults:** `/Users/r/Projects/ascii research/alexharri-ascii/website_repo/website-master/src/components/AsciiScene/sampling/gpu/GPUSamplingDataGenerator.ts` (line 115-116)
- **Shader:** `/Users/r/Projects/ascii research/alexharri-ascii/website_repo/website-master/src/components/AsciiScene/sampling/gpu/shaders.ts` (lines 272, 282, 301, 318)
- **CPU variant:** `/Users/r/Projects/ascii research/alexharri-ascii/website_repo/website-master/src/components/AsciiScene/sampling/cpu/generateSamplingData.ts` (function `crunchSamplingVector`)

---

## 5. WebGL Shaders

### Location
`/Users/r/Projects/ascii research/alexharri-ascii/website_repo/website-master/src/components/AsciiScene/sampling/gpu/shaders.ts`

### Shader Programs

#### 1. Passthrough Vertex Shader (lines 3-11)
```glsl
#version 300 es
in vec2 a_position;
out vec2 v_texCoord;
void main() {
  gl_Position = vec4(a_position, 0.0, 1.0);
  v_texCoord = a_position * 0.5 + 0.5;
}
```

#### 2. Sampling Fragment Shader (lines 13-134)
- Samples lightness at circular regions using Vogel's method (golden angle spiral)
- Supports quality levels (1-16 subsamples per circle)
- Uses Rec. 709 luma weights: `vec3(0.2126, 0.7152, 0.0722)`
- Golden angle constant: `3.883222077450933`

Key uniforms:
- `u_samplingPoints[numCircles]` - Circle center positions
- `u_circleRadius` - Circle radius
- `u_samplingQuality` - Number of subsamples

#### 3. Max Value Fragment Shader (lines 136-185)
- Computes maximum value across all circles in each cell
- Used as input for global crunch

#### 4. Directional Crunch Fragment Shader (lines 262-288)
- Applies directional crunch effect using external points
- Uniform: `u_directionalCrunchExponent`

#### 5. Global Crunch Fragment Shader (lines 290-324)
- Applies global crunch effect using cell max values
- Uniform: `u_globalCrunchExponent`

#### 6. External Max Fragment Shader (lines 186-259)
- Computes max values for external sampling points

#### 7. Copy Fragment Shader (lines 327-340)
- Simple texture copy utility

### GPU Generator Class
`/Users/r/Projects/ascii research/alexharri-ascii/website_repo/website-master/src/components/AsciiScene/sampling/gpu/GPUSamplingDataGenerator.ts`

Manages the multi-pass rendering pipeline:
1. Raw sampling pass
2. External max pass  
3. Directional crunch pass (optional)
4. Max value pass
5. Global crunch pass (optional)

---

## Summary Table

| Parameter | Value | Source |
|-----------|-------|--------|
| Alphabet | default.json (95 chars, 6D) | `alphabets/default.json` |
| circleRadius | 0.28125 | `alphabets/default.json` |
| Sampling Points | 6 circles | `alphabets/default.json` |
| External Points | 10 points | `alphabets/default.json` |
| globalCrunchExponent | 3 (default) | `GPUSamplingDataGenerator.ts` |
| directionalCrunchExponent | 7 (default) | `GPUSamplingDataGenerator.ts` |
| Luma Weights | 0.2126, 0.7152, 0.0722 | `shaders.ts` |
| Golden Angle | 3.883222077450933 | `shaders.ts` |

---

## Additional Findings

### Available Alphabet Variants
1. **default.json** - 6 samples, 95 chars, full feature set
2. **six-samples.json** - 6 samples, alternative positions
3. **two-samples.json** - 2 samples, simplified
4. **pixel-short.json** - 1 sample, short character set
5. **simple-directional-crunch.json** - 6 samples, simplified external points

### Key Files for Port
- `/Users/r/Projects/ascii research/alexharri-ascii/website_repo/website-master/src/components/AsciiScene/alphabets/default.json` - Character vectors
- `/Users/r/Projects/ascii research/alexharri-ascii/website_repo/website-master/src/components/AsciiScene/sampling/gpu/shaders.ts` - GLSL shaders
- `/Users/r/Projects/ascii research/alexharri-ascii/website_repo/website-master/src/components/AsciiScene/sampling/gpu/GPUSamplingDataGenerator.ts` - GPU pipeline
- `/Users/r/Projects/ascii research/alexharri-ascii/website_repo/website-master/src/components/AsciiScene/sampling/cpu/generateSamplingData.ts` - CPU pipeline reference
- `/Users/r/Projects/ascii research/alexharri-ascii/website_repo/website-master/src/components/AsciiScene/characterLookup/KdTree.ts` - k-d tree for matching
- `/Users/r/Projects/ascii research/alexharri-ascii/website_repo/website-master/src/components/AsciiScene/characterLookup/CharacterMatcher.ts` - Character matching with cache
