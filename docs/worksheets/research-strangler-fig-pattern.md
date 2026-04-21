> **STATUS: ACTIVE REFERENCE** — Strangler fig migration pattern analysis for incremental porting.

# Strangler Fig Pattern for Incremental C++ to Rust Game Porting

## Executive Summary

This document provides a comprehensive guide to applying the Strangler Fig pattern for incrementally migrating a C++ game engine to Rust. The pattern enables safe, gradual replacement of system components without requiring a dangerous big-bang rewrite, allowing the game to remain functional throughout the migration process.

## Table of Contents

1. [The Strangler Fig Pattern Explained](#1-the-strangler-fig-pattern-explained)
2. [Core Principles for Game Engine Migration](#2-core-principles-for-game-engine-migration)
3. [Application to C++ to Rust Porting](#3-application-to-c-to-rust-porting)
4. [Real-World Examples and Case Studies](#4-real-world-examples-and-case-studies)
5. [Handling Shared State Between Old and New Code](#5-handling-shared-state-between-old-and-new-code)
6. [Incremental Rendering Replacement Strategy](#6-incremental-rendering-replacement-strategy)
7. [Testing During Migration](#7-testing-during-migration)
8. [Implementation Roadmap](#8-implementation-roadmap)
9. [Risks and Mitigations](#9-risks-and-mitigations)

---

## 1. The Strangler Fig Pattern Explained

### 1.1 Origin and Metaphor

The Strangler Fig pattern takes its name from the strangler fig tree (*Ficus aurea*), a plant that germinates in the crown of a host tree. As it grows, it sends roots down toward the ground while branches spread toward the canopy. Eventually, the fig tree's roots encircle the host tree, and the fig becomes self-sustaining while the original host dies, leaving a hollow structure in the shape of the former tree.

In software, this pattern describes the gradual replacement of a legacy system by building new functionality around the existing one, until the old system is entirely supplanted. The key insight is that replacement happens incrementally, with both systems running in parallel, rather than attempting a complete rewrite.

### 1.2 Core Definition

> The Strangler Fig pattern incrementally migrates a legacy system by gradually replacing specific pieces of functionality with new applications and services. As you replace features from the legacy system, the new system eventually comprises all of the old system's features. This approach suppresses the old system so that you can decommission it.

— Microsoft Azure Architecture Center

### 1.3 When to Use This Pattern

The Strangler Fig pattern is particularly valuable when:

- **Continuous operation is required**: The game cannot be taken offline for months-long rewrites
- **Feature development must continue**: New features and bug fixes must ship during migration
- **The codebase is large**: Complete rewrites of large codebases (100K+ lines) are inherently risky
- **Hidden coupling exists**: Shared databases, authentication, and internal APIs create dependencies that aren't immediately visible
- **Historical knowledge is limited**: The original developers may no longer be available to explain every behavior and edge case

### 1.4 Why Big-Bang Rewrites Fail

Research and industry experience consistently demonstrate that complete rewrites fail more often than they succeed:

| Failure Mode | Description |
|--------------|-------------|
| **Scope creep** | New requirements accumulate during the rewrite, extending timelines indefinitely |
| **Behavioral drift** | Subtle edge cases and undocumented behaviors are lost in translation |
| **Opportunity cost** | The team cannot ship new features while focused on the rewrite |
| **Integration risk** | A single cutover point creates massive deployment risk |
| **Knowledge loss** | The "tribal knowledge" embedded in the existing code is never fully captured |

The Strangler Fig pattern directly addresses these failure modes by treating migration as a series of small, reversible, testable changes rather than one monolithic effort.

---

## 2. Core Principles for Game Engine Migration

### 2.1 The Five Phases

The Strangler Fig pattern applied to game engine migration follows these phases:

1. **Identify the migration boundary**: Determine where you can intercept calls between the legacy C++ code and the new Rust code
2. **Create an abstraction layer**: Define a stable interface that both systems can communicate through
3. **Implement the new system alongside**: Build the Rust replacement while the C++ system continues running
4. **Route traffic incrementally**: Gradually shift functionality from C++ to Rust, starting with the safest components
5. **Remove the legacy system**: Once all functionality is migrated, remove the C++ code and the abstraction layer

### 2.2 Key Architectural Decisions

#### 2.2.1 Choose the Right Interception Point

The interception point is where you redirect calls from the old system to the new one. For a game engine, natural interception points include:

- **Rendering pipeline**: The draw calls that push geometry to the GPU
- **Input handling**: The layer that receives windowing and input events
- **File I/O**: Asset loading and saving operations
- **Audio subsystem**: Sound effect and music playback
- **Physics calculation**: Collision detection and response

#### 2.2.2 Define Clear Ownership Boundaries

Each subsystem should have a clear owner:

- **Greenfield in Rust**: New systems that have no C++ equivalent
- **Migrated**: Systems being actively replaced
- **Legacy**: Systems still running in C++
- **Shared**: Systems that require coordination between both languages

### 2.3 Traffic Routing Strategies

There are three primary strategies for routing traffic during migration:

| Strategy | Description | Risk Level | Use Case |
|----------|-------------|-------------|----------|
| **Decorator/Proxy** | Rust wraps C++ calls, adding functionality before/after | Low | Adding logging, profiling |
| **Gateway** | Central router sends requests to either C++ or Rust | Medium | Feature-level switching |
| **Branch by Abstraction** | Abstraction layer abstracts the implementation detail | Medium | Internal component replacement |

For game engines, a hybrid approach is typically best: gateway-level routing for major subsystems (rendering, audio), and branch by abstraction for internal components.

---

## 3. Application to C++ to Rust Porting

### 3.1 The CXX Bridge Library

The primary tool for C++/Rust interop is [CXX](https://cxx.rs/), a library that provides safe interoperability between Rust and C++ code. Unlike traditional FFI approaches, CXX generates type-safe bindings that eliminate entire categories of bugs.

#### 3.1.1 Key Concepts in CXX

CXX distinguishes between three kinds of types at the FFI boundary:

- **Shared structs**: Data structures whose fields are visible to both languages
- **Opaque types**: Types where only one language has access to internals
- **Vocabulary types**: Standard types like `String`, `Vec`, `Box`, and `Result` that are natively understood by both languages

```rust
#[cxx::bridge]
mod ffi {
    // Shared struct - visible to both languages
    struct GameTransform {
        position: [f32; 3],
        rotation: [f32; 4],
        scale: [f32; 3],
    }

    // Opaque Rust type - C++ can use it but can't see inside
    extern "Rust" {
        type RustEntity;
        fn entity_get_transform(entity: &RustEntity) -> GameTransform;
    }

    // Opaque C++ type - Rust can use it but can't see inside  
    extern "C++" {
        type CppRenderBackend;
        fn backend_submit(backend: Pin<&mut CppRenderBackend>, frame: &RenderFrame);
    }
}
```

#### 3.1.2 When to Use CXX vs. Bindgen

| Tool | Use Case |
|------|----------|
| **CXX** | Bidirectional interop, complex types, need for zero-copy passing |
| **Bindgen** | One-way C++ wrapping, simpler interfaces, C compatibility required |

### 3.2 FFI Boundary Design Principles

When designing the FFI boundary between C++ and Rust for a game engine:

1. **Minimize the surface area**: Keep the FFI boundary small and focused
2. **Use value types for hot paths**: Pass simple structs by value to avoid allocation
3. **Prefer owned types at boundaries**: Clarify ownership to prevent leaks
4. **Represent errors explicitly**: Use `Result` types rather than error codes
5. **Version the interface**: Plan for the interface to evolve

### 3.3 Practical Interop Patterns

#### 3.3.1 Calling Rust from C++

```rust
// In Rust: expose functions to C++
#[cxx::bridge]
mod game_ffi {
    extern "Rust" {
        fn init_engine(config: &EngineConfig) -> Result<Box<Engine>>;
        fn engine_tick(engine: Pin<&mut Engine>, dt: f32);
    }
}
```

```cpp
// In C++: call the Rust functions
#include "game_ffi.h"

int main() {
    EngineConfig config = {/* ... */};
    auto engine = init_engine(config).unwrap();
    
    while (true) {
        engine_tick(*engine, 0.016f);
    }
}
```

#### 3.3.2 Calling C++ from Rust

```rust
// In Rust: wrap C++ classes
#[cxx::bridge]
mod renderer_ffi {
    extern "C++" {
        type VulkanBackend;
        type RenderPass;
        
        fn VulkanBackend_new() -> Box<VulkanBackend>;
        fn VulkanBackend_begin_frame(
            backend: Pin<&mut VulkanBackend>, 
            width: u32, 
            height: u32
        ) -> Result<RenderPass>;
    }
}
```

#### 3.3.3 Shared Data Structures

```rust
#[cxx::bridge]
mod shared {
    // C++-compatible layout - no padding issues
    #[repr(C)]
    pub struct Vec3 {
        pub x: f32,
        pub y: f32,
        pub z: f32,
    }

    // Arrays are supported
    pub struct MeshData {
        pub vertices: Vec<Vec3>,
        pub indices: Vec<u32>,
    }
}
```

### 3.4 Rust Adoption Feasibility Assessment

Based on the Rust Foundation's interop initiative research, the following scenarios are feasible for incremental Rust adoption:

| Scenario | Feasibility |
|----------|-------------|
| **Interprocess boundaries** | High - processes can be migrated one at a time |
| **Small, simple FFI surface** | High - manually manageable API boundaries |
| **Rich C++ APIs** | Medium - requires significant abstraction |
| **Large monolithic codebases** | Medium-High - depends on existing modularity |

For game engines, the rendering subsystem is often a good candidate for early migration because it tends to have clear boundaries and well-defined interfaces.

---

## 4. Real-World Examples and Case Studies

### 4.1 librsvg: C to Rust Incremental Migration

The librsvg project successfully migrated from C to Rust incrementally over several years. Key learnings:

- **Started with small, isolated modules**: Initial Rust code replaced low-risk components
- **Maintained C API compatibility**: The external interface remained unchanged
- **Used cargo to build both**: The build system handled both languages seamlessly
- **Gradually expanded scope**: More C code was converted as confidence grew

The project demonstrates that incremental migration at the library level is viable even without complex FFI infrastructure.

### 4.2 Rustybuzz: C++ to Rust Text Shaping

Rustybuzz is a Rust implementation of the HarfBuzz text shaping library, originally written in C++. The migration:

- **Followed the original API closely**: Minimized behavioral differences
- **Ran both implementations in parallel**: Validated output matching
- **Migrated function by function**: Replaced individual shaping operations

This pattern is directly applicable to game engine subsystems like text rendering.

### 4.3 Quake 3: C to Rust Translation

The Immunant team successfully translated Quake 3 (a substantial C codebase) to Rust using the C2Rust translator. While not incremental in the Strangler Fig sense, this demonstrates:

- **Automated translation is possible**: Tools exist to handle mechanical conversions
- **Manual refinement is required**: Generated code needs idiomatic Rust rewriting
- **Performance is maintained**: Translated code can match original performance

### 4.4 Game Server Migration: Microservices Approach

Research on game server migration (IDS University study) demonstrates the Strangler Fig pattern applied to game servers:

1. **Created automated tests first**: Validated existing functionality before any changes
2. **Defined API specification**: Clear contract between old and new systems
3. **Used Branch by Abstraction**: Incrementally replaced DAO classes with remote connectors
4. **Ran in parallel**: Both systems processed the same requests, results were compared

This approach is directly applicable to game client migration.

### 4.5 Alloy-rs: Game Engine Reimplementation

Alloy-rs is a Rust reimplementation of the C++ Alloy game engine. The project shows:

- **Complete rewrite is one valid approach**: Sometimes starting fresh is right
- **Rust idioms can improve design**: Not everything needs 1:1 translation
- **Community contributions accelerate progress**: Open source enables collaborative migration

---

## 5. Handling Shared State Between Old and New Code

### 5.1 Types of Shared State

In a game engine context, shared state falls into several categories:

| Category | Examples | Migration Strategy |
|----------|---------|-------------------|
| **Configuration** | Game settings, rendering parameters | Migrate first, read-only access |
| **Runtime data** | Entity transforms, game state | Dual-write, careful synchronization |
| **GPU resources** | Textures, shaders, buffers | Shared ownership, reference counting |
| **Memory pools** | Asset allocators | Migrate ownership sequentially |

### 5.2 Synchronization Patterns

#### 5.2.1 Dual-Write Pattern

Both C++ and Rust write to shared state, allowing validation:

```
C++: entity.set_position(new_pos) ----> Shared State <---- Rust: entity.set_position(new_pos)
                                                                                    |
                                                                                    v
                                                                      Validate: positions match
```

This pattern is expensive but ensures correctness during migration.

#### 5.2.2 Write-Through Migration

One language is the authoritative source; the other receives updates:

```
C++: is_authoritative = true
Rust: is_authoritative = false

Write: C++ writes directly
Read:  Both can read, C++ is authoritative
Sync:  C++ pushes updates to Rust
```

#### 5.2.3 Event Sourcing

Changes are communicated as events rather than direct state mutation:

```
C++: emit EntityMoved { id, new_position }
       |
       v
Event Bus ---> C++ Entity Handler (processes events)
       |
       v  
Rust Entity Handler (mirrors state)
```

This pattern is particularly powerful for game state synchronization.

### 5.3 Memory Management Across the FFI

#### 5.3.1 Ownership Strategies

Rust's ownership model must be carefully considered at FFI boundaries:

| Strategy | Description | When to Use |
|---------|-------------|-------------|
| **Owned** | Transfer ownership across boundary | Clear ownership transfer (e.g., asset loading) |
| **Borrowed** | Temporary reference | Read-only data, hot paths |
| **Shared ownership** | `Rc`/`Arc` or raw pointers | Complex lifecycles, caches |
| **Copy** | Clone for small types | Simple data (vectors, transforms) |

#### 5.3.2 Avoiding Use-After-Free

The most common FFI bug is accessing memory that has been freed:

```rust
// BAD: Returning borrowed data that C++ might free
#[cxx::bridge]
mod bad_example {
    extern "Rust" {
        fn get_temp_transform() -> Transform; // Will dangle!
    }
}

// GOOD: Transfer ownership
#[cxx::bridge]
mod good_example {
    extern "Rust" {
        fn create_transform() -> Transform; // C++ owns the returned value
    }
}

// GOOD: Keep data in Rust, let C++ borrow temporarily
#[cxx::bridge]
mod also_good {
    extern "Rust" {
        type TransformStore;
        fn store_get_transform(store: &TransformStore, id: u32) -> &Transform;
    }
}
```

### 5.4 Data Structure Migration

#### 5.4.1 Step 1: Identify Data Dependencies

Map out which data structures are accessed by which subsystems:

```
Entity Component System (ECS):
  - TransformComponent: Used by Physics, Rendering, AI
  - RenderComponent: Used by Rendering only
  - PhysicsComponent: Used by Physics only
  
Migration Priority:
  1. TransformComponent (high coupling)
  2. PhysicsComponent (clear boundaries)
  3. RenderComponent (can be replaced independently)
```

#### 5.4.2 Step 2: Define Translation Layer

Create explicit conversions between C++ and Rust representations:

```rust
// Translation layer between C++ and Rust entity formats
impl From<ffi::EntityData> for EntityData {
    fn from(cpp: ffi::EntityData) -> Self {
        Self {
            id: EntityId(cpp.id),
            transform: cpp.transform.into(),
            velocity: cpp.velocity.map(|v| v.into()),
            // Explicit conversion prevents hidden coupling
        }
    }
}

impl From<EntityData> for ffi::EntityData {
    fn from(rust: EntityData) -> Self {
        Self {
            id: rust.id.0,
            transform: rust.transform.into(),
            velocity: rust.velocity.map(|v| v.into()),
        }
    }
}
```

#### 5.4.3 Step 3: Implement Write Consolidation

Once a component is fully migrated, consolidate writes:

```rust
// Phase 1: Both C++ and Rust can write
// Phase 2: Rust becomes authoritative, C++ only reads
// Phase 3: Remove C++ access entirely

pub enum AuthoritativeSource<T> {
    Legacy(T),    // C++ is authoritative
    Modern(T),    // Rust is authoritative
}

impl<T> AuthoritativeSource<T> {
    pub fn migrate(&mut self) {
        if let AuthoritativeSource::Legacy(data) = self {
            *self = AuthoritativeSource::Modern(std::mem::take(data));
        }
    }
}
```

---

## 6. Incremental Rendering Replacement Strategy

### 6.1 Why Start with Rendering?

The rendering subsystem is often the best candidate for early migration because:

1. **Clear boundaries**: Well-defined interfaces to the rest of the engine
2. **Independent testing**: Can verify visual output without game logic
3. **GPU isolation**: Rendering bugs rarely cause memory safety issues
4. **Performance-critical**: Rust's zero-cost abstractions provide benefits here

### 6.2 Migration Order

For an ASCII/artistic game engine like Asciicker, suggested migration order:

```
1. Font/Glyph System (lowest risk)
   └─> ASCII renderer
        └─> Texture atlas manager
             └─> Scene graph (transform hierarchy)
                  └─> Post-processing effects
                       └─> Main rendering pipeline
```

### 6.3 Step-by-Step Implementation

#### Phase 1: Create the Abstraction Layer

```rust
// Rust: Define the rendering trait
pub trait RenderBackend {
    fn begin_frame(&mut self, width: u32, height: u32) -> RenderResult;
    fn draw_sprite(&mut self, sprite: &Sprite, transform: &Transform);
    fn end_frame(&mut self) -> RenderResult;
}

// C++: Implement wrapper that delegates to Rust
class RustRenderBackend {
    std::unique_ptr<rust::RenderBackend> backend;
public:
    void begin_frame(uint32_t w, uint32_t h) {
        backend->begin_frame(w, h).unwrap();
    }
    void draw_sprite(const Sprite& s, const Transform& t) {
        backend->draw_sprite(&s, &t);  // Convert to FFI types
    }
};
```

#### Phase 2: Create the Switch Mechanism

```rust
// Configuration-driven backend selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBackendType {
    Legacy,  // C++ backend
    Modern,  // Rust backend
}

impl RenderConfig {
    pub fn backend(&self) -> RenderBackendType {
        // Feature flag enables gradual rollout
        if self.enable_rust_renderer {
            RenderBackendType::Modern
        } else {
            RenderBackendType::Legacy
        }
    }
}
```

#### Phase 3: Implement the Rust Backend

```rust
// New Rust rendering implementation
pub struct RustRenderBackend {
    renderer: BevyRenderer,
    frame_buffers: Vec<FrameBuffer>,
    // ... state
}

impl RenderBackend for RustRenderBackend {
    fn begin_frame(&mut self, width: u32, height: u32) -> RenderResult {
        // Pure Rust implementation
        // Can use Bevy, raw wgpu, etc.
    }
    
    fn draw_sprite(&mut self, sprite: &Sprite, transform: &Transform) {
        // Implementation
    }
}
```

#### Phase 4: Incremental Feature Migration

```rust
// Migrate features one at a time
pub struct RenderFeatures {
    pub enable_batching: bool,
    pub enable_instancing: bool,
    pub enable_post_processing: bool,
}

impl RenderConfig {
    pub fn features(&self) -> RenderFeatures {
        RenderFeatures {
            enable_batching: self.feature_level >= FeatureLevel::Advanced,
            enable_instancing: self.feature_level >= FeatureLevel::Advanced,
            enable_post_processing: self.feature_level >= FeatureLevel::Full,
        }
    }
}
```

### 6.4 Handling Graphics API Differences

C++ game engines often use OpenGL, DirectX, or Vulkan directly. When migrating to Rust:

| Strategy | Pros | Cons |
|----------|------|------|
| **Use existing C++ graphics API** | No rewriting, proven | Can't use Rust graphics ecosystem |
| **Wrap C++ backend in Rust** | Keeps GPU code in C++ | Less idiomatic Rust |
| **Port to wgpu/Bevy** | Modern Rust, cross-platform | Requires rewriting GPU code |
| **Use gfx-hal** | Low-level control | High boilerplate |

For Asciicker's ASCII rendering needs, wrapping the existing C++ rendering and gradually migrating to Bevy or raw wgpu is likely the best approach.

---

## 7. Testing During Migration

### 7.1 Testing Strategy Overview

Testing during incremental migration requires a multi-layered approach:

```
┌─────────────────────────────────────────────────────────────┐
│                    End-to-End Tests                         │
│         (Game runs, visual output matches)                  │
├─────────────────────────────────────────────────────────────┤
│                  Integration Tests                          │
│      (C++ ↔ Rust communication works correctly)            │
├─────────────────────────────────────────────────────────────┤
│                   Component Tests                           │
│     (Rust components produce correct outputs)              │
├─────────────────────────────────────────────────────────────┤
│                     FFI Tests                               │
│   (Data roundtrips C++ → Rust → C++ unchanged)             │
└─────────────────────────────────────────────────────────────┘
```

### 7.2 Parallel Run Testing

The most important testing pattern during migration is **parallel run**:

1. **Run both implementations** on the same input
2. **Compare outputs** at every step
3. **Log discrepancies** without crashing
4. **Verify before switching** authoritative source

```rust
pub struct ParallelRun<T> {
    legacy: T,
    modern: T,
    compare_fn: fn(&T, &T) -> bool,
}

impl<T> ParallelRun<T> {
    pub fn process(&mut self, input: &Input) -> Output {
        let legacy_output = self.legacy.process(input);
        let modern_output = self.modern.process(input);
        
        if !self.compare_fn(&legacy_output, &modern_output) {
            error!("Output mismatch: legacy={:?}, modern={:?}", 
                   legacy_output, modern_output);
            // In debug: panic. In production: log and use legacy.
        }
        
        // Gradually shift to modern as confidence builds
        if should_use_modern() {
            modern_output
        } else {
            legacy_output
        }
    }
}
```

### 7.3 Property-Based Testing

Rust's `proptest` library enables property-based testing, which is valuable for migration:

```rust
proptest! {
    #[test]
    fn test_transform_composition(a in any::<Transform>(), 
                                  b in any::<Transform>(),
                                  c in any::<Transform>()) {
        // In C++: (a * b) * c
        // In Rust: a.compose(b).compose(c)
        
        let cpp_result = cpp::compose(
            cpp::compose(a.clone(), b.clone()), 
            c.clone()
        );
        let rust_result = a.compose(b).compose(c);
        
        prop_assert!(approx_eq(cpp_result, rust_result));
    }
}
```

### 7.4 Golden File Testing for Rendering

For rendering subsystems, golden file testing compares visual output:

```rust
#[test]
fn test_ascii_render_matches_cpp() {
    let scene = load_test_scene("test_level.ascii");
    
    // Render with C++
    let cpp_output = cpp_renderer.render(&scene);
    cpp_output.save("golden/cpp_render.png");
    
    // Render with Rust
    let rust_output = rust_renderer.render(&scene);
    rust_output.save("golden/rust_render.png");
    
    // Compare (allow small differences for floating point)
    let diff = image_diff(&cpp_output, &rust_output);
    assert!(diff < 0.01, "Render difference: {}", diff);
}
```

### 7.5 Contract Testing

Contract testing ensures the FFI boundary maintains invariants:

```rust
#[cxx::bridge]
mod contract_tests {
    extern "Rust" {
        #[test]
        fn transform_roundtrips(transform: Transform) -> bool;
        
        #[test]  
        fn entity_id_stays_valid_after_ffi(entity: Entity) -> bool;
    }
}
```

### 7.6 Metrics and Observability

During migration, instrument both systems:

```rust
pub struct MigrationMetrics {
    pub legacy_latency_us: Histogram,
    pub modern_latency_us: Histogram,
    pub output_mismatch_count: Counter,
    pub migration_progress: Gauge,
}

impl MigrationMetrics {
    pub fn record_comparison(&self, legacy: &Output, modern: &Output) {
        let matches = legacy == modern;
        self.output_mismatch_count.inc_by(!matches as u64);
        
        // Log first N mismatches for debugging
        if !matches && self.output_mismatch_count.get() < 10 {
            error!("Mismatch: {:?}", (legacy, modern));
        }
    }
}
```

---

## 8. Implementation Roadmap

### 8.1 Phase 1: Foundation (Weeks 1-4)

| Week | Task | Deliverable |
|------|------|-------------|
| 1 | Set up CXX build integration | Project compiles with both C++ and Rust |
| 2 | Define core shared types | `Transform`, `Vec3`, `EntityId` in FFI |
| 3 | Create C++ wrapper for Rust calls | Basic glue code compiles |
| 4 | Implement "Hello World" subsystem | Minimal Rust code runs via C++ |

### 8.2 Phase 2: First Migration (Weeks 5-10)

| Week | Task | Deliverable |
|------|------|-------------|
| 5-6 | Migrate asset loading system | Rust loads, C++ consumes |
| 7-8 | Migrate font/glyph system | Text rendering in Rust |
| 9-10 | Run parallel, validate output | Both systems produce identical output |

### 8.3 Phase 3: Core Systems (Weeks 11-20)

| Week | Task | Deliverable |
|------|------|-------------|
| 11-14 | Migrate rendering pipeline | Frame rendering in Rust |
| 15-17 | Migrate entity system | ECS in Rust |
| 18-20 | Migrate physics | Physics in Rust |

### 8.4 Phase 4: Polish and Remove (Weeks 21-24)

| Week | Task | Deliverable |
|------|------|-------------|
| 21-22 | Migrate remaining systems | Full coverage |
| 23 | Remove C++ code | Pure Rust engine |
| 24 | Performance optimization | Benchmark and tune |

### 8.5 Milestone Criteria

Each phase should meet these criteria before proceeding:

- [ ] All tests pass (legacy behavior preserved)
- [ ] Parallel run shows < 0.1% output differences
- [ ] Performance meets or exceeds C++ baseline
- [ ] No regression in test coverage
- [ ] Documentation updated for new architecture

---

## 9. Risks and Mitigations

### 9.1 Technical Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| **FFI performance overhead** | High | Medium | Profile early; optimize hot paths; minimize boundary crossings |
| **Memory safety in C++** | Medium | High | Audit C++ code; add runtime checks; use sanitizers in testing |
| **Behavioral differences** | High | High | Parallel run testing; golden file comparison; property testing |
| **Build complexity** | Medium | Medium | Use CXX's build system; document build process; automate CI |

### 9.2 Process Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| **Migration fatigue** | High | Medium | Regular milestones; celebrate wins; rotate teams |
| **Scope creep** | High | High | Clear boundaries; frozen C++ API; prioritize ruthlessly |
| **Feature freeze** | Medium | Medium | New features can use Rust; maintain C++ for hot fixes |
| **Knowledge silos** | Medium | Medium | Pair programming; documentation; knowledge sharing |

### 9.3 Mitigation Strategies

#### 9.3.1 Establish SLOs (Service Level Objectives)

```yaml
# Migration SLOs
migration:
  availability: 99.9%  # No regressions in availability
  correctness: 100%     # Output must match exactly
  performance: 95%     # Modern must be >= 95% of legacy performance
  
# Rollback triggers
rollback_if:
  - correctness_drift > 0.1%
  - availability < 99.5%
  - latency_increase > 20%
```

#### 9.3.2 Implement Kill Switches

```rust
pub struct MigrationConfig {
    pub enable_rust_physics: bool,
    pub enable_rust_rendering: bool,
    pub enable_rust_assets: bool,
    
    // Global override - instant rollback
    pub rust_enabled: bool,
}

impl MigrationConfig {
    pub fn from_env() -> Self {
        Self {
            enable_rust_physics: std::env::var("RUST_PHYSICS").is_ok(),
            enable_rust_rendering: std::env::var("RUST_RENDERING").is_ok(),
            enable_rust_assets: std::env::var("RUST_ASSETS").is_ok(),
            rust_enabled: std::env::var("RUST_ENABLED").unwrap_or_default() != "false",
        }
    }
}
```

#### 9.3.3 Maintain Feature Parity Checklist

Before declaring a subsystem migrated:

- [ ] All original features work identically
- [ ] All bug fixes from C++ have been ported
- [ ] Performance is acceptable (benchmark attached)
- [ ] Tests cover the new implementation
- [ ] Documentation is updated
- [ ] No remaining C++ dependencies except FFI layer

---

## Appendix A: Recommended Tools and Libraries

| Category | Tool | Purpose |
|----------|------|---------|
| **FFI Bridge** | CXX | Safe Rust/C++ interop |
| **Build System** | Cargo + cxx-build | Integrated builds |
| **Testing** | proptest | Property-based testing |
| **Image Comparison** | image crate | Golden file testing |
| **Metrics** | metrics crate | Observability |
| **ECS** | bevy_ecs | Entity Component System |
| **Graphics** | wgpu / Bevy | GPU rendering |

---

## Appendix B: Key Resources

- [Martin Fowler's Strangler Fig Article](https://martinfowler.com/bliki/StranglerFigApplication.html)
- [Microsoft Azure: Strangler Fig Pattern](https://learn.microsoft.com/en-us/azure/architecture/patterns/strangler-fig)
- [CXX Documentation](https://cxx.rs/)
- [Thoughtworks: Embracing Strangler Fig](https://www.thoughtworks.com/en-us/insights/articles/embracing-strangler-fig-pattern-legacy-modernization)
- [AWS: Branch by Abstraction](https://docs.aws.amazon.com/prescriptive-guidance/latest/modernization-decomposing-monoliths/branch-by-abstraction.html)
- [Rust Foundation Interop Initiative](https://rust-lang.github.io/rust-project-goals/2025h1/seamless-rust-cpp.html)

---

## Appendix C: Quick Reference Card

```
STRANGLER FIG MIGRATION CHECKLIST
==================================

Before Starting:
[ ] Architecture documented
[ ] FFI boundaries identified
[ ] Shared types defined
[ ] Test strategy in place
[ ] Rollback plan prepared

During Migration:
[ ] One subsystem at a time
[ ] Parallel run enabled
[ ] Metrics collecting
[ ] Differences logged
[ ] Regular validation

Phase Completion:
[ ] Tests pass
[ ] Output matches
[ ] Performance acceptable
[ ] Documentation updated
[ ] Ready for next phase
```

---

*Document Version: 1.0*  
*Created: February 2026*  
*Applicable to: C++ to Rust Game Engine Porting*
