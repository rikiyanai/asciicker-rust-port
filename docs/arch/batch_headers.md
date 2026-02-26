# Header File Function Analysis

Batch analysis of four core header files: `enemygen.h`, `fast_rand.h`, `lexer.h`, `matrix.h`.

## enemygen.h

### `fast_srand` (fast_rand.h:6-8)

**Signature:**
```c
inline void fast_srand(int seed)
```

**Purpose:**
Seeds the global pseudorandom number generator state. Sets `g_seed` to the provided integer value.

**Called by:**
- No callers found via grep (initialization is implicit at module load, or called before first `fast_rand()`)

**Calls:**
- None (direct assignment only)

**Globals read:**
- None

**Globals mutated:**
- `g_seed` — Global static seed state (line 3)

**Side effects:**
- Changes the deterministic sequence of all subsequent `fast_rand()` calls

**Notes:**
- Intentionally simple: no validation or error handling
- Caller is responsible for ensuring seed is initialized before first `fast_rand()` call
- Default initial value: `g_seed = 0x57654321` (hardcoded at module scope)


### `fast_rand` (fast_rand.h:13-16)

**Signature:**
```c
inline int fast_rand(void)
```

**Purpose:**
Returns a pseudorandom 15-bit integer in the range [0, 32767] using a linear congruential generator (LCG).

**Called by:**
- `game.cpp` (weapon selection in spawn logic, equipment probability checks)
- `asciiid.cpp` (editor-level randomization)

**Calls:**
- None (direct arithmetic only)

**Globals read:**
- `g_seed` — Current generator state

**Globals mutated:**
- `g_seed` — Updated via LCG formula (line 14)

**Side effects:**
- Advances the global RNG state deterministically; repeated calls return a sequence

**Notes:**
- LCG parameters: multiplier=214013, increment=2531011 (Microsoft Visual C++ defaults)
- Bit-shift logic: `(g_seed >> 16) & 0x7FFF` extracts bits 16-30 for better distribution
- `FAST_RAND_MAX` defined as `0x7fff` (32767) — returned values in [0, 32767]
- No seed initialization guard; caller must call `fast_srand()` first
- Usage in `EnemyGen` struct: equipment probabilities computed as `fast_rand() % 11 < threshold`


---

## fast_rand.h

See **enemygen.h** section above (this file contains only `fast_srand()` and `fast_rand()`).


---

## lexer.h

### `Lexer::Get` (lexer.h:138-1222)

**Signature:**
```c
uint32_t Get(char c)
```

**Purpose:**
Processes a single input character and returns a token type (low 8 bits) plus optional retroactive recolor count (high 8+ bits). Implements a ~40-state finite state machine for JavaScript-like syntax highlighting in terminal script cells.

**Called by:**
- `game.cpp` — Terminal text rendering uses `Lexer::Get()` to colorize script cells during `PaintProc`
- Recursive re-entry within `Get()` itself when rescanning after state transitions

**Calls:**
- `Matcher::find()` (static member function) — Performs binary-prefix keyword matching (line 376, 856, 887)
- Recursive `Get(c)` — When rescanning after Pure state re-entry (lines 799, 874, 903, 919, 940, 981, 1006, 1014, 1022, 1042, 1055, 1066, 1074, 1217)
- Standard library: `strcmp()` (indirectly via `std::sort` in Matcher initialization)

**Globals read:**
- `Matcher::init` (static, first-call flag for keyword table initialization)
- `Matcher::match[]` (static keyword table, ~70 entries including JS reserved words and engine keywords "ak"/"akPrint")
- `Matcher::index[256]` (static first-char index for O(1) coarse keyword lookup)

**Globals mutated:**
- `state` (member, 8-bit current parser state)
- `depth` (member, 8-bit template expression nesting depth)
- `idxlen` (member, 16-bit dual-purpose: keyword matcher state OR escape digit count)
- `call` (member, 32-bit shifted whitespace/identifier length before '(')

**Side effects:**
- Maintains incremental parsing state across character-by-character input
- Retroactively recolors prior characters on certain transitions (e.g., '/' becomes comment marker)
- Accumulates function-call metadata in `call` field for retroactive coloring of identifiers before '('
- Template expression depth tracking for nested `${ ... }` expressions

**Notes:**
- **State machine scope**: ~40 states (line 50-88) covering strings (single/double/template) with escape sequences, comments, numbers (dec/oct/hex/bin/float), identifiers, keywords, and template literals
- **Keyword matching**: Sorted binary-prefix table (Matcher::find, line 147-253). Engine-specific keywords: "ak", "akPrint" (line 170) marked with TODO that they should have distinct coloring but don't
- **Recolor encoding**: High bits (>>8) encode count of characters to retroactively recolor; example: `block_comment | (1<<8)` means "token is block_comment, recolor 1 prior char"
- **Template handling**: Tracks `depth` for nested `${ expr }` — depth increases when `${` encountered (line 430), decreases when `}` closes expression (line 313)
- **Ambiguous transitions**: Pure state dispatches on 6 character types that require lookahead: '/' (division vs //, /*), '.' (member vs float), '0' (0x, 0b, octal, or plain 0), '{' (error if inside template), '}' (close template or plain brace), identifier (becomes keyword or stays identifier after next char)
- **TODO(PIPELINE-FIX)** (line 119-121): `idxlen` overloading (keyword matcher idx|len vs escape digit count) — a keyword inside a string escape could corrupt matcher state (not reachable in practice)
- **TODO(PIPELINE-FIX)** (line 192-197): Matcher index[] initialization memset only covers `size*sizeof(uint16_t)` bytes but array is 256 entries — tail entries uninitialized if `size < 256`, safe because of entry-path checks but confusing footgun
- **TODO(PIPELINE-FIX)** (line 155-157): "ak" and "akPrint" are mixed in with JS keywords but should have another color — currently all get same keyword color in game.cpp


### `Lexer::Matcher::find` (lexer.h:147-253)

**Signature:**
```c
static uint16_t find(uint16_t state, char c)
```

**Purpose:**
Performs incremental binary-prefix keyword matching. Given a current matcher state (`idx | (len<<idx_bits)`) and a new character `c`, returns the next matcher state or `0xFFFF` if no match.

**Called by:**
- `Get()` (line 376, 856, 887) — Three call sites for keyword identification

**Calls:**
- `std::sort()` (line 206) — One-time initialization to sort keyword table alphabetically
- `strncmp()` (line 249) — Prefix comparison to advance through sorted table
- `strlen()` (line 210) — Validation of keyword length against `max_len`
- `memset()` (line 207) — Initialize index[] array
- `assert()` (lines 150, 200, 210) — Debug validation

**Globals read:**
- `match[]` (static keyword table, const, sorted once on first call)
- `index[256]` (static first-char lookup, initialized once)

**Globals mutated:**
- `init` (static flag, set false on first call to prevent re-initialization)
- `match[]` (sorted in-place on first call)
- `index[256]` (populated on first call)

**Side effects:**
- Lazy one-time initialization of keyword table and index (first invocation of `find()` across entire lexer lifetime)
- Modifies static state globally; subsequent calls operate on cached sorted table and index

**Notes:**
- **Bit packing**: `state` parameter encodes `idx` (9 bits, up to 512 keywords) in low bits and `len` (6 bits, up to 63-char keywords) in bits 9-14; bit 15 indicates partial match (1) vs exact match (0)
- **Binary search optimization**: First char uses direct `index[c]` lookup (O(1)) to jump to first keyword starting with that letter (line 216-222). Subsequent chars perform linear scan with early exit based on alphabetical ordering (line 231-249)
- **Early exit**: If `match[idx][len] > c`, no later entry can match (alphabetical invariant), so loop breaks (line 241-245)
- **Return convention**: Exact match returns `idx | (len<<idx_bits) | 0` (bit 15 clear); partial match returns `idx | (len<<idx_bits) | (1<<15)` (bit 15 set); no match returns `0xFFFF`
- **Assertion safety**: `assert(state != 0xffff)` at entry (line 150) prevents double-matching after a failed search
- **Keyword table**: ~70 entries (line 158-183) including JS reserved words (break, case, if, etc.), built-in types (Array, Object, Promise, etc.), global functions (parseInt, eval, etc.), engine keywords ("ak", "akPrint"), and error constructors


---

## matrix.h

### `Invert` (matrix.h:55-191)

**Signature:**
```c
template <typename M>
bool Invert(const M m[16], M invOut[16])
```

**Purpose:**
Computes the 4x4 matrix inverse using cofactor expansion (adjugate / determinant method). Returns `false` if matrix is singular (det == 0).

**Called by:**
- `render.cpp` — View-projection matrix inversion for unprojection (screen coords to world ray)
- `physics.cpp` — Inverse transforms for collision response

**Calls:**
- None (pure arithmetic)

**Globals read:**
- None

**Globals mutated:**
- None

**Side effects:**
- Outputs computed inverse matrix to `invOut[16]`

**Notes:**
- **Column-major layout**: Input/output matrices are 4x4 stored as 16-element flat arrays in column-major (OpenGL convention): `m[col*4 + row]`
- **Algorithm**: Cofactor expansion computes all 16 cofactor minors (3x3 determinants), then divides by the full 4x4 determinant. Output is `adj(m) / det(m)` where `adj(m)` is the adjugate (transpose of cofactor matrix)
- **Determinant computation** (line 177): `det = m[0]*inv[0] + m[1]*inv[4] + m[2]*inv[8] + m[3]*inv[12]` — reuses cofactor column already computed, avoiding a separate determinant calculation
- **Scalar division optimization** (line 185): `det = 1.0 / det` followed by 16 multiplies is cheaper than 16 divisions
- **Singularity guard**: Returns `false` if `det == 0` (line 179-180); no exception thrown
- **Works for any invertible matrix**: Not limited to rotation matrices; handles scale, shear, and arbitrary transforms
- **Footgun**: No numerical stability checks; ill-conditioned matrices may produce inaccurate results


### `MatProduct` (matrix.h:199-220)

**Signature:**
```c
template <typename M>
void MatProduct(const M a[16], const M b[4], M ab[16])
```

**Purpose:**
Multiplies two 4x4 matrices: `ab = a * b`. Used to compose transforms (model-view-projection chains).

**Called by:**
- `render.cpp` — Chaining model, view, and projection transforms

**Calls:**
- None (pure arithmetic)

**Globals read:**
- None

**Globals mutated:**
- None

**Side effects:**
- Outputs result to `ab[16]`

**Notes:**
- **Column-major layout**: All inputs and outputs in column-major (OpenGL convention)
- **Manual unrolling**: 16 explicit dot-product statements (lines 201-219) instead of loops; allows compiler to vectorize with SIMD
- **Column-wise decomposition**: Each output column `ab[col*4..col*4+3]` is computed as `a * b_column` (dot product of matrix `a` rows with column `col` of matrix `b`)
- **Semantics**: `ab[0..3]` = `a[0..3] . b[0..3]` (first result column), etc.


### `Product` (matrix.h — 4x4 * 4-vector) (matrix.h:228-234)

**Signature:**
```c
template <typename M, typename V, typename MV>
void Product(const M m[16], const V v[4], MV mv[4])
```

**Purpose:**
Transforms a 4-vector by a 4x4 matrix: `mv = m * v`. Supports mixed-precision (e.g., double matrix * float vector).

**Called by:**
- `render.cpp` — Transforms world-space vertices into clip-space

**Calls:**
- None (pure arithmetic)

**Globals read:**
- None

**Globals mutated:**
- None

**Side effects:**
- Outputs transformed vector to `mv[4]`

**Notes:**
- **Three template parameters**: `M` (matrix element type), `V` (input vector type), `MV` (output type) allow mixed-precision arithmetic
- **Explicit casts**: `(MV)` cast on each component (line 230-233) prevents implicit narrowing warnings
- **Column-major matrix**: Input matrix `m` is column-major; multiplication follows standard linear algebra: output element `i` = dot(row_i of m, v)
- **Homogeneous coordinates**: Works on 4-vectors including the `w` (homogeneous) component for proper perspective transforms


### `Product` (matrix.h — 4-element dot product) (matrix.h:255-258)

**Signature:**
```c
template <typename L, typename R>
auto Product(const L l[4], const R r[4])
```

**Purpose:**
Computes a 4-element dot product. Returns auto-deduced type (e.g., `float` if both inputs are `float`).

**Called by:**
- Used for homogeneous plane-distance tests (frustum culling, half-space tests)

**Calls:**
- None (pure arithmetic)

**Globals read:**
- None

**Globals mutated:**
- None

**Side effects:**
- None (returns computed value)

**Notes:**
- **4-element (not 3-element)**: Includes the homogeneous `w` component for plane equations and half-space tests
- **Auto-deduced return type**: Template argument deduction determines return type from `L*R` arithmetic
- **Usage**: Plane-distance test: `dot(plane, vertex) > 0` checks if vertex is on positive side of plane


### `TransposeProduct` (matrix.h:243-249)

**Signature:**
```c
template <typename M, typename V, typename MV>
void TransposeProduct(const M m[16], const V v[4], MV mv[4])
```

**Purpose:**
Multiplies a transposed matrix by a vector: `mv = transpose(m) * v`. For orthonormal rotation matrices, transpose equals inverse, avoiding full matrix inversion cost.

**Called by:**
- `render.cpp` — Inverse-rotate directions and normals without full `Invert()` call

**Calls:**
- None (pure arithmetic)

**Globals read:**
- None

**Globals mutated:**
- None

**Side effects:**
- Outputs result to `mv[4]`

**Notes:**
- **Avoids matrix inversion**: For rotation matrices (orthonormal), `transpose(m) == inverse(m)`, so this is cheaper than calling `Invert()`
- **Column-major interpretation**: `TransposeProduct(m, v)` computes `transpose(m) * v` by accessing `m` in row-major order while treating it as column-major (swapping index access)
- **Mixed precision support**: Same as `Product(m, v)` — three template parameters for flexible type mixing


### `PositiveProduct` (matrix.h:264-267)

**Signature:**
```c
template <typename L, typename R>
inline int PositiveProduct(L l[4], R r[4])
```

**Purpose:**
Tests if dot product is positive; returns 1 if `dot(l, r) > 0`, else 0. Used for frustum culling half-space tests.

**Called by:**
- `render.cpp` — Frustum plane culling in terrain/world render stages

**Calls:**
- `Product(l, r)` (4-element dot product, line 266)

**Globals read:**
- None

**Globals mutated:**
- None

**Side effects:**
- None (returns 0 or 1)

**Notes:**
- **Frustum culling**: Vertex is on positive side of frustum plane iff `dot(plane, vertex) > 0`
- **Boolean return**: Returns `int` (0 or 1) instead of `bool` for bitwise operations in culling masks


### `Rotation` (matrix.h:280-307)

**Signature:**
```c
template <typename V, typename A, typename M>
inline void Rotation(const V v[3], A a, M m[16])
```

**Purpose:**
Converts axis-angle representation (unit axis `v[3]` and angle `a` in radians) to a 4x4 rotation matrix using Rodrigues' formula.

**Called by:**
- `render.cpp` — Camera orbit rotation around terrain focus point
- `physics.cpp` — Rotating collision geometry

**Calls:**
- `cos()` (line 282), `sin()` (line 283) — Standard C math functions

**Globals read:**
- None

**Globals mutated:**
- None

**Side effects:**
- Outputs rotation matrix to `m[16]`

**Notes:**
- **Rodrigues' formula**: `R = cos(a)*I + sin(a)*[v]× + (1-cos(a))*(v⊗v)` where `[v]×` is skew-symmetric cross-product matrix, `v⊗v` is outer product
- **Assumes unit axis**: Caller MUST normalize `v[3]` before calling; no validation check. Non-unit axis silently produces non-orthonormal matrix with scale drift
- **Optimization**: Precomputes `c = cos(a)`, `s = sin(a)`, `d = 1-cos(a)` (line 282-286) to avoid recomputation
- **Column-major output**: Matrix is 4x4 in column-major, with 0s in bottom row and right column (lines 291, 296, 301, 303-306)
- **TODO(PIPELINE-FIX)**: No normalization check on `v`; non-unit axis causes scale drift in chained transforms


### `RandRot` (matrix.h:321-329)

**Signature:**
```c
template <typename R, typename M>
inline void RandRot(R r[3], M m[16])
```

**Purpose:**
Generates a uniformly distributed random rotation matrix from 3 uniform random values in [0,1] using Arvo's method (Graphics Gems III). **STUB: Not implemented.**

**Called by:**
- No callers found via grep (function body is empty)

**Calls:**
- None (function body is empty)

**Globals read:**
- None

**Globals mutated:**
- None

**Side effects:**
- **BUG**: Function does nothing; output matrix `m[16]` is uninitialized

**Notes:**
- **Algorithm (intended, not implemented)**: Arvo's method from "Fast Random Rotation Matrices" composes random XY-plane rotation with Householder reflection for uniform coverage of SO(3)
- **Why this method**: Standard Euler-angle randomization clusters rotations near poles; Arvo's method gives uniform distribution
- **TODO(PIPELINE-FIX)**: Stub function — no implementation. Callers relying on this get uninitialized output matrix `m[16]`
- **Comments describe intended algorithm** (line 323-328) but code body is empty


### `CrossProduct` (matrix.h:335-340)

**Signature:**
```c
template <typename V>
inline void CrossProduct(const V a[3], const V b[3], V ab[3])
```

**Purpose:**
Computes 3D cross product: `ab = a × b`.

**Called by:**
- `matrix.h` — `RayIntersectsTriangle()` (line 381-385, 408-412)
- `matrix.h` — `PlaneFromPoints()` (line 451)
- `matrix.h` — `SphereIntersectTriangle()` (line 466)
- `render.cpp`, `terrain.cpp`, `physics.cpp` (implicit via collision/rendering functions)

**Calls:**
- None (pure arithmetic)

**Globals read:**
- None

**Globals mutated:**
- None

**Side effects:**
- Outputs cross product to `ab[3]`

**Notes:**
- **Standard definition**: `ab[0] = a[1]*b[2] - a[2]*b[1]` (and cyclic permutations)
- **Result properties**: Perpendicular to both `a` and `b`; magnitude = |a| * |b| * sin(angle between them)
- **Used for normals**: Cross product of two edge vectors gives face normal


### `DotProduct` (matrix.h:344-347)

**Signature:**
```c
template <typename V>
inline V DotProduct(const V a[3], const V b[3])
```

**Purpose:**
Computes 3D dot product: scalar = `a · b`.

**Called by:**
- `matrix.h` — `RayIntersectsTriangle()` (line 388-389, 402-403, 415-416, 423-424)
- `matrix.h` — `PlaneFromPoints()` (line 452)
- `matrix.h` — `SphereIntersectTriangle()` (lines 468, 474-476, 480-481, 485-486, 496-498, 502, 507, 512)
- `render.cpp`, `terrain.cpp`, `physics.cpp` (collision and rendering)

**Calls:**
- None (pure arithmetic)

**Globals read:**
- None

**Globals mutated:**
- None

**Side effects:**
- None (returns scalar)

**Notes:**
- **Standard definition**: `a · b = a[0]*b[0] + a[1]*b[1] + a[2]*b[2]`
- **3-element (not 4-element)**: Unlike the 4-element `Product(l[4], r[4])` which operates on homogeneous coordinates
- **Result properties**: Scalar; positive if angle < 90°, zero if perpendicular, negative if angle > 90°
- **Usage in collision**: Computing distances and half-space tests


### `RayIntersectsTriangle` (matrix.h:374-444)

**Signature:**
```c
inline bool RayIntersectsTriangle(
  double ray[10], double v0[3], double v1[3], double v2[3], double ret[3],
  bool positive_only = false, double* out_u = 0, double* out_v = 0
)
```

**Purpose:**
Moller-Trumbore ray-triangle intersection test. Returns `true` if ray hits triangle, updating `ray[9]` to closest hit distance and outputting hit point and barycentric coordinates.

**Called by:**
- `asciiid.cpp` — Picking: raycast from camera through mouse pixel for object selection
- `physics.cpp` — Collision detection: test movement rays against terrain mesh

**Calls:**
- None (pure arithmetic)

**Globals read:**
- None

**Globals mutated:**
- Modifies `ray[9]` (write-back to progressive nearest-hit on successful intersection)

**Side effects:**
- Updates `ray[9]` to tighter max-t clamp if intersection found closer than previous
- Outputs hit point to `ret[3]` and barycentric coordinates to `out_u`, `out_v` (if pointers provided)

**Notes:**
- **Ray layout (ray[10])**:
  - `ray[0..2]`: Unused padding (reserved for homogeneous point compatibility)
  - `ray[3..5]`: Ray direction vector (need not be normalized)
  - `ray[6..8]`: Ray origin point
  - `ray[9]`: Current closest hit distance (acts as max-t clamp)
- **TODO(PIPELINE-FIX)**: Unused padding at `ray[0..2]` is confusing footgun; non-obvious 10-element layout should use named struct or call-site comments
- **Moller-Trumbore algorithm**: Computes barycentric coordinates `u, v` directly without computing triangle plane first; requires 1 division, 2 cross products, 3 dot products
- **Progressive nearest-hit**: On successful intersection, `ray[9]` is clamped to `t` (line 436), so subsequent triangle tests automatically reject farther hits. Caller accumulates closest hit across all triangles in single pass
- **EPSILON = 0.0000001**: Parallel-ray detection threshold (line 376)
- **Barycentric output**: `u, v` enable texture coordinate or color interpolation at hit point; third barycentric `w = 1 - u - v`
- **Optional parameters**: `positive_only` (default false) rejects hits with `t < 0` (line 426-427); `out_u`, `out_v` pointers (default null) store barycentric coordinates


### `PlaneFromPoints` (matrix.h:447-453)

**Signature:**
```c
template <typename V>
inline void PlaneFromPoints(const V a[3], const V b[3], const V c[3], V p[4])
```

**Purpose:**
Computes a plane equation from 3 points. Outputs plane as `p[4]` where `p[0..2]` is the normal and `p[3]` is the distance component.

**Called by:**
- `physics.cpp`, collision code (implicit via frustum/plane tests)

**Calls:**
- `CrossProduct()` (line 451) — Computes normal from edge vectors
- `DotProduct()` (line 452) — Computes plane distance

**Globals read:**
- None

**Globals mutated:**
- None

**Side effects:**
- Outputs plane equation to `p[4]`

**Notes:**
- **Algorithm**: Compute two edge vectors `u = b - a`, `v = c - a`. Normal is `p[0..2] = u × v`. Distance is `p[3] = -dot(normal, a)`
- **Plane equation**: `dot(p, point) = 0` for all points on the plane; `> 0` for points on positive side, `< 0` for negative side
- **Normal direction**: Depends on winding order (right-hand rule); if `a, b, c` are counterclockwise from viewer, normal points outward
- **Homogeneous plane**: `p[4]` is the full homogeneous plane representation for use with 4-vector dot products


### `SphereIntersectTriangle` (matrix.h:455-516)

**Signature:**
```c
inline bool SphereIntersectTriangle(
  float S[4]/*center,radius*/, float v0[3], float v1[3], float v2[3]
)
```

**Purpose:**
Tests if a sphere collides with a triangle. Returns `true` if sphere center plus radius intersects the triangle surface or edges.

**Called by:**
- `physics.cpp` — Collision detection: sphere vs terrain mesh

**Calls:**
- `CrossProduct()` (line 466) — Computes triangle normal
- `DotProduct()` (lines 468, 474-476, 480-481, 485-486, 496-498, 502, 507, 512) — Distance and containment tests

**Globals read:**
- None

**Globals mutated:**
- None

**Side effects:**
- None (returns boolean)

**Notes:**
- **Sphere layout**: `S[0..2]` is center point, `S[3]` is radius
- **Algorithm**: 
  1. Translate triangle vertices relative to sphere center (lines 457-459)
  2. Compute triangle normal and distance from center to plane (lines 465-469)
  3. Reject if sphere too far from plane (line 471-472)
  4. Test sphere vs each vertex (lines 474-487)
  5. Test sphere vs each edge (lines 489-513) using closest-point-on-segment logic
- **Early exit**: Multiple rejection tests (vertex/edge proximity) for efficiency
- **Vertices as arrays**: A, B, C are relative-to-sphere positions; AB, AC, BC are edge vectors
- **Scalar fields**: e1, e2, e3 are edge vector squared-magnitudes; Q1, Q2, Q3 are closest points on edges
- **Footgun**: No comments in code; algorithm is complex 3D geometry (GJK-style separation axis tests)

