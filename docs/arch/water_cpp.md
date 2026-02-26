# Asciicker Water System Architecture

Date: 2026-02-20  
Source: `/Users/r/Downloads/asciicker-Y9-2/water.cpp`  
Related Files: `physics.h`, `physics.cpp`, `terrain.h`, `docs/skills/physics-system/SKILL.md`

## Overview

The Asciicker water system is currently in a **design planning phase**. The file `water.cpp` contains architectural specifications and mathematical foundations for a future water rendering implementation, but no actual water surface rendering code has been implemented yet. The existing water functionality in the engine is limited to basic terrain material identification (Material 0 = water) and simplified physics interaction through the physics system.

This document provides a comprehensive analysis of the water system architecture based on the design notes in `water.cpp`, the existing physics integration, and the terrain material system.

---

## 1. Water Surface Rendering

### Current Status

The water surface rendering system is **not implemented**. The `water.cpp` file explicitly states:

```
Current status: DESIGN NOTES ONLY — no actual water rendering implemented
Water patches would overlay terrain patches with animated, reflective surface effects.
```

### Proposed Rendering Architecture

The design documents propose water as a **sparse low-poly mesh overlay on terrain**. The key constraint is performance: the design specifies that there should be "no more than 10 triangles in viewport" to maintain acceptable rendering performance.

#### Design Goals

The water surface rendering system aims to achieve several objectives. First, it should function as a sparse mesh scattered over the terrain in a manner similar to object meshes but with instancing support. Second, the water must have animated, reflective surface effects that create visual appeal. Third, the system needs to overlay existing terrain patches without replacing them. Finally, it must support efficient visibility determination to avoid rendering occluded water triangles.

#### Prerequisite Systems

The design notes clearly state that **object instancing must exist before water can be implemented**. This is because water reflections need to reflect both terrain and objects. The prerequisite systems include an OBJ importer for mesh loading and pure mesh support with a single UV channel (normals, colors, and smooth groups are explicitly not required).

### 3-Pass Rendering Pipeline Proposal

The proposed rendering pipeline consists of three distinct passes designed to efficiently handle water reflections.

**Pass 1: Base Scene Rendering**  
The first pass renders the terrain and object instances to the framebuffer. This creates the scene that will later be reflected in the water surface.

**Pass 2: Water Triangle Visibility Determination**  
The second pass performs a depth test to determine which water triangle cells are visible. Only visible water triangles are linked into a temporary list for further processing. This optimization prevents wasted work on occluded water triangles.

**Pass 3: Reflection Rendering**  
The third pass processes each visible water triangle by calculating the reflection transform and computing bounding planes. The system then gathers and renders reflections of terrain patches and object instances, clipping the geometry by the computed planes.

### Water as Terrain Material

In the current implementation, water is treated as ** 0** inMaterial the terrain system. The default terrain initialization creates a 12x12 grass center with a 2-patch water border. The water material uses specific glyphs for visual representation:

- Ramp 0: `,` (calm water)
- Ramp 1: ` ` (still water)
- Ramp 2: `!` (agitated water)
- Ramp 3: ` ` (waterfall)

The default terrain height is set to `0xA000` (40960), which is above the water level at `0x8000` (32768). This ensures that the playable terrain is visible and not submerged beneath the water surface.

---

## 2. Water Reflection and Refraction

### Reflection Mathematics

The water.cpp design notes contain detailed mathematical formulations for computing reflections across a water plane. This is fundamental to the entire water rendering system.

#### Reflection Plane Transformation

Given a reflection plane defined by the equation `A*x + B*y + C* z + D == 0`, a point `P = [x, y, z, 1]` is transformed to its reflection using the formula:

```
[x, y, -2*(A*x + B*y + D)/C - z, 1]
```

#### Reflection Transformation Matrix

When a full transformation matrix is needed, the design specifies the following matrix representation:

```
[  1     0     0     0  ]
[  0     1     0     0  ]
[-2A/C -2B/C  -1   -2D/C]
[  0     0     0     1  ]
```

This matrix transforms points from world space into reflection space, where the Z coordinate is negated relative to the water plane.

### Cascaded Reflection Design

The reflection system uses a **cascaded reflection** approach that involves multiple clipping planes to optimize the rendered reflection area. The design specifies several types of clipping planes that work together to efficiently determine which geometry needs to be rendered in the reflection.

**Viewport Edge Planes**  
The four planes defining the viewport edges are reflected across the water plane to create corresponding reflection clipping planes. These ensure that only geometry visible within the viewport is rendered in the reflection.

**Mirror Boundary Planes**  
Planes are constructed from the viewing vector and the mirror boundary edges. These planes help handle the geometric distortion that occurs when viewing a reflection at oblique angles.

**Mirror Plane**  
The water surface itself serves as the reflection plane, and geometry is mirrored across this plane during rendering.

### Sprite Deformation in Reflections

A notable feature in the design notes is the acknowledgment that **sprites can be deformed** in reflections. When the reflection plane's normal in view coordinates has a non-zero X coordinate (indicating water flow that appears somewhat horizontal on screen), sprites would need to be rendered z-column by z-column, adjusting each column's screen-space Y position. This creates a more realistic distortion effect for sprites reflected in the water.

### Refraction Considerations

The design notes do not explicitly address refraction (the bending of light through water). However, the 3-pass pipeline structure could potentially support refraction in a future implementation by rendering a third pass with appropriate distortion applied to the terrain beneath the water surface.

---

## 3. Wave Animation

### Current Status

Wave animation is **not implemented** in the current codebase. The design notes specify that water patches would have "animated" surface effects, but no specific animation system has been designed or implemented.

### Design Considerations

Based on the architecture proposals, wave animation would likely be implemented through the following mechanisms:

**Vertex Displacement**  
As a low-poly mesh overlay, water triangles would have their vertices displaced over time to create wave motion. The sparse nature of the mesh (maximum 10 triangles in viewport) suggests that vertex animation would be computationally feasible.

**Material Animation**  
The terrain material system already supports different glyphs for water (`,`, ` `, `!`). A wave animation system could cycle through these glyphs based on time and position to create the appearance of rippling water.

**Integration with Terrain**  
Since water overlays terrain patches, the wave animation would need to be synchronized with the underlying terrain rendering to ensure visual consistency.

---

## 4. Water Collision and Physics

### Physics Integration

The water physics system is **partially implemented** through the physics module. The integration uses the `PhysicsIO` structure defined in `physics.h`.

#### Water Field in PhysicsIO

The `PhysicsIO` structure includes a `water` field that represents the water surface Z coordinate:

```cpp
// WHY water: Water surface Z coordinate for buoyancy calculation.
// Physics uses this to determine if character is submerged and apply upward
// buoyant force (Archimedes principle). Set by game.cpp based on terrain water level.
// Units: world Z coordinate (same as pos[2]).
float water;    // Water surface Z (for buoyancy, set by game.cpp)
```

This field is set by `game.cpp` before calling the `Animate()` function, based on the terrain water level.

### Buoyancy Implementation

When a character's position `pos[2]` is below the water surface (`water > pos[2]`), the physics system applies buoyancy forces. According to the physics system documentation:

```
When `water > pos[2]`, gravity is reduced by buoyancy. Physics behavior changes dramatically at water boundary.
```

The buoyancy force counteracts gravity, creating the effect of reduced weight when submerged. This is a simplified implementation of Archimedes' principle.

### Physics Constants for Water

The physics system defines specific constants for water-based movement:

| Constant | Value | Description |
|----------|-------|-------------|
| Max velocity (water) | 10 units/sec | Maximum speed when submerged |
| Max velocity (air) | 27 units/sec | Maximum speed when airborne |
| Gravity | ~9.8 units/sec² | Standard gravitational acceleration |
| Timestep | 15ms fixed | Physics simulation rate (~66 Hz) |

The reduced max velocity in water (10 units/sec vs 27 units/sec in air) reflects the increased drag that characters experience when moving through water.

### Grounded Detection and Water

The physics system uses `accum_contact_z >= 1.0` to determine grounded state. When a character is in water, this detection still functions but interacts with the buoyancy system to determine whether the character can jump or perform other ground-based actions.

### Jump Mechanics

The jump flag (`io.jump`) is consumed by physics when applied, provided the character is grounded. In water, the reduced effective gravity may affect jump behavior, though the exact implementation details would be in `physics.cpp`.

---

## 5. Spatial Query System

### Design Requirements

The water system design specifies the need for efficient geometric queries to support both rendering and gameplay interactions:

**Ray Casting**  
The system requires ray-to-triangle intersection queries for player looking at water surface. This is needed for detecting when the player is looking at and potentially interacting with the water.

**Frustum Culling**  
Visibility queries using clipping planes are required to determine which geometry needs reflection rendering. The cascaded reflection approach relies on efficient plane-based queries to avoid rendering occluded geometry.

### Query Implementation Requirements

The design notes indicate that points, triangles, polygons, and planes need to be stored in a way that supports fast queries. While the exact data structure is not specified, the terrain system's quadtree and the world system's BSP tree provide models for how this might be implemented.

---

## 6. Porting Considerations for Rust

### Implementation Complexity

Porting the water system to Rust would involve addressing several challenges:

**Design-to-Implementation Gap**  
The water system exists primarily as design notes rather than working code. The Rust port would need to complete the design work, transforming the architectural specifications into functional code.

**Reflection Rendering Pipeline**  
Implementing the 3-pass rendering pipeline would require integration with the existing terrain and world rendering systems. The reflection pass would need to efficiently render the scene twice (once for the main view, once for reflection).

**Geometry Query System**  
The spatial queries needed for water (ray casting, frustum culling) would require leveraging or extending the existing terrain quadtree and world BSP tree implementations.

**Performance Constraints**  
The design specifies a maximum of 10 water triangles in the viewport. This constraint would need to be maintained in the Rust implementation to ensure acceptable performance.

### Recommended Approach

A phased implementation approach would be most effective:

**Phase 1: Basic Water Material**  
Extend the terrain material system to support animated water glyphs and refine the visual appearance of water surfaces.

**Phase 2: Physics Integration**  
Ensure the existing buoyancy system works correctly and add any missing physics interactions (swimming vs. wading, dive mechanics, etc.).

**Phase 3: Reflection Rendering**  
Implement the 3-pass pipeline with efficient water triangle visibility determination and reflection rendering.

**Phase 4: Wave Animation**  
Add vertex and material animation for wave effects, maintaining the performance constraints.

---

## 7. File References

### Primary Source Files

| File | Lines | Purpose |
|------|-------|---------|
| `/Users/r/Downloads/asciicker-Y9-2/water.cpp` | 71 | Design notes only - no implementation |
| `/Users/r/Downloads/asciicker-Y9-2/physics.h` | 123 | PhysicsIO interface with water field |
| `/Users/r/Downloads/asciicker-Y9-2/terrain.h` | 219 | Terrain patch/material system |
| `/Users/r/Downloads/asciicker-Y9-2/docs/skills/physics-system/SKILL.md` | 86 | Physics system documentation |

### Related Systems

The water system interacts with multiple other engine systems. The terrain system provides the underlying heightfield and material grid. The world system provides object geometry that must be reflected. The physics system handles buoyancy and water collision. The render system would need to implement the water rendering pipeline.

---

## 8. Summary

The Asciicker water system represents an ambitious future enhancement to the engine. While no actual water surface rendering is implemented, the design notes provide a clear architectural vision involving a sparse low-poly mesh overlay with cascaded reflections. The existing water functionality is limited to basic terrain material identification (Material 0) and simplified buoyancy physics through the `PhysicsIO` structure.

Key findings from this research include the following: water surface rendering is unimplemented and exists only as design documentation proposing a 3-pass rendering pipeline; reflection mathematics are fully specified with transformation matrices and cascaded reflection plane calculations; wave animation is not designed beyond the concept of animated surface effects; and water collision/physics is partially implemented through the `water` field in `PhysicsIO` with buoyancy forces applied when characters are submerged.

The Rust port would benefit from a clear implementation roadmap that addresses the design-to-implementation gap while maintaining the performance constraints specified in the original design.
