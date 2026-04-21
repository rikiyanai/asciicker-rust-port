> **STATUS: ACTIVE REFERENCE** — Bevy framework analysis, February 2026. Target version: Bevy 0.18+.

# Bevy Engine Research Report

**Date:** February 2026
**Purpose:** Research for Asciicker Rust Port

---

## 1. Official GitHub Repository

| Item | Details |
|------|---------|
| **Repository URL** | https://github.com/bevyengine/bevy |
| **Stars** | ~44,600+ |
| **Forks** | ~4,300+ |
| **License** | Apache 2.0 |
| **Primary Language** | Rust (94.2%) |
| **Latest Release** | v0.18.0 (January 13, 2026) |
| **Contributors** | 430+ |
| **Organization** | Bevy Engine (@bevyengine) |

---

## 2. Key Features and Architecture

### Entity Component System (ECS)

Bevy is built entirely around the **ECS paradigm**, which separates data from behavior:

- **Entities:** Unique identifiers that hold no data themselves
- **Components:** Rust structs containing data (e.g., `Position`, `Velocity`)
- **Systems:** Rust functions that process entities with specific component combinations

### Bevy ECS Features

| Feature | Description |
|---------|-------------|
| Queries | Efficient filtering and iteration over entity subsets |
| Global Resources | Singleton data accessible across all systems |
| Local Resources | Per-system scoped data |
| Change Detection | Track when component data mutates |
| Parallel Scheduler | Lock-free parallel execution of systems |

### Rendering

- **2D Renderer:** Sprite sheets, dynamic texture atlases, cameras, materials
- **3D Renderer:** Lights, shadows, meshes, GLTF loading, PBR materials
- **Render Graph:** Backend-agnostic, modular, composable render pipelines

### Other Features

- Cross-platform: Windows, macOS, Linux, Web, iOS, Android
- Hot reloading for rapid iteration
- Fast compile times (0.8-3.0s with fast compile config)
- Asset management system
- UI framework
- Audio system
- Physics integration (via Avian physics engine)
- State machine support

---

## 3. Why Bevy is Popular in Rust Gamedev

### Key Reasons

1. **Pure Rust Implementation**
   - Built entirely in Rust from the ground up
   - No dependencies on C/C++ engines
   - Leverages Rust's memory safety and zero-cost abstractions

2. **Modern ECS Architecture**
   - Data-driven design encourages clean, decoupled code
   - Excellent for games with many entities (particles, enemies, etc.)
   - Memory-efficient and cache-friendly data layout
   - Natural parallelism

3. **Active Development**
   - Frequent releases (~quarterly major versions)
   - Strong community contributions (430+ contributors)
   - Growing ecosystem of plugins and assets

4. **Performance**
   - Comparable to Unity DOTS and sometimes faster
   - No garbage collection pauses
   - Predictable performance characteristics

5. **Freedom from Licensing Concerns**
   - Completely free and open-source (Apache 2.0)
   - No runtime fees (avoiding Unity-style controversies)
   - No vendor lock-in

6. **Developer Experience**
   - Rust's type system catches bugs at compile time
   - IDE support via rust-analyzer
   - Growing learning resources and tutorials

---

## 4. Current Version and Latest Updates

### Version History

| Version | Release Date | Notes |
|---------|--------------|-------|
| v0.18.0 | January 13, 2026 | Latest stable release |
| v0.14.0 | June 2024 | Major refactor |
| v0.13.0 | Early 2024 | Various improvements |

### Recent Development Focus

- Continued rendering improvements
- Better WebAssembly (WASM) support
- Mobile platform support enhancements
- Performance optimizations
- API stability improvements

---

## 5. Comparisons to Other Engines

### Bevy vs Other Rust Engines

| Engine | Type | Bevy Advantage |
|--------|------|----------------|
| **ggez** | 2D-focused | More feature-complete, ECS-based |
| **macroquad** | Immediate mode | Simpler, but less scalable |
| **Fyrox** | 3D-focused | More mature editor, less active dev |
| **Piston** | Framework | More modular, but less cohesive |

### Bevy vs Major Game Engines

| Aspect | Bevy | Unity | Unreal Engine | Godot |
|--------|------|-------|---------------|-------|
| **Language** | Rust | C# | C++ | GDScript/C# |
| **Architecture** | ECS | OOP/ECS hybrid | OOP | OOP |
| **License** | Apache 2.0 | Proprietary | Proprietary | MIT |
| **Editor** | In development | Mature | Mature | Mature |
| **Performance** | Excellent | Good (DOTS) | Excellent | Good |
| **2D Support** | Good | Excellent | Limited | Excellent |
| **3D Support** | Good | Excellent | Excellent | Good |
| **Learning Curve** | Moderate | Easy | Steep | Easy |
| **Runtime Fees** | None | Yes (controversial) | Yes | None |

### Performance Benchmarks (from 2025 comparison)

| Scenario | Bevy | Unity DOTS | Unreal |
|----------|------|------------|--------|
| 10K entities | 120 FPS | 110 FPS | 95 FPS |

### When to Choose Bevy

**Choose Bevy if:**
- You want pure Rust development
- You're building data-heavy games (many entities)
- You prefer code-first approach over visual editors
- You need cross-platform (including web) deployment
- You want to avoid runtime fees
- You're comfortable with Rust

**Consider alternatives if:**
- You need a mature visual editor
- You need AAA-level 3D features
- You need platform-specific native features
- You prefer visual scripting
- You need faster initial prototyping

---

## 6. Ecosystem and Resources

### Official Resources
- Website: https://bevy.org
- GitHub: https://github.com/bevyengine/bevy
- Discord: Active community

### Community Assets
- bevy-assets: Community plugins and examples (~1,000+ stars)
- Awesome Bevy: Curated list of projects
- Various tutorials and books

### Related Projects
- **Avian:** Physics engine for Bevy
- **Space Editor:** Visual editor for Bevy
- **Godot Rust bindings:** For using Rust with Godot

---

## 7. Summary

Bevy is the leading pure Rust game engine, distinguished by its modern ECS architecture, excellent performance, and fully open-source nature. With ~44K GitHub stars and active development, it represents a viable choice for Rust game developers seeking a data-driven approach. The v0.18.0 release (January 2026) represents the latest stable version with ongoing improvements to rendering, platform support, and developer experience.

For a Rust port of Asciicker, Bevy's ECS architecture would be well-suited for handling game entities, and its 2D rendering capabilities would provide a solid foundation.

---

*Research compiled from GitHub, official Bevy website, and various 2025 comparisons.*
