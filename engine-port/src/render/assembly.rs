//! A3D-to-runtime assembly system.
//!
//! Watches for loaded A3D assets and builds runtime structures:
//! - `RuntimeTerrain` from `A3dTerrain` patches
//! - `RuntimeWorld` from `A3dWorld` instances
//! - `RuntimeMaterials` from `MaterialTable` (standalone Bevy Resource)
//!
//! Also queues AKM mesh loads for world mesh instances.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::asset_loader::a3d_terrain::A3dTerrain;
use crate::asset_loader::a3d_terrain::MatCell as AssetMatCell;
use crate::asset_loader::a3d_terrain::MaterialTable;
use crate::asset_loader::a3d_world::A3dWorld;
use crate::asset_loader::akm_mesh::AkmMesh;
use crate::asset_loader::bevy_loaders::A3dFile;
use crate::render::material::{MatCell as RenderMatCell, Material};
use crate::terrain::RuntimeTerrain;
use crate::terrain::shadow::{default_light_dir, update_terrain_dark};
use crate::world::RuntimeWorld;

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Tracks the A3D asset loading and assembly state.
#[derive(Resource, Default)]
pub struct AssemblyState {
    /// Handle to the A3D file being loaded (set by `load_a3d_scene`).
    pub a3d_handle: Option<Handle<A3dFile>>,
    /// Whether the assembly has completed.
    pub assembled: bool,
}

/// Registry of AKM mesh handles and loaded mesh data.
#[derive(Resource, Default)]
pub struct MeshRegistry {
    /// Pending mesh handles keyed by mesh name.
    pub meshes: HashMap<String, Handle<AkmMesh>>,
    /// Loaded mesh data keyed by mesh name.
    pub loaded: HashMap<String, AkmMesh>,
}

/// Standalone Bevy Resource wrapping the material table.
///
/// Resolves HIGH gap #5: MaterialTable must be available as a Bevy Resource
/// for the render pipeline to call resolve_to_grid with real materials.
#[derive(Resource)]
pub struct RuntimeMaterials(pub Vec<Material>);

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Startup system that loads the default A3D scene file.
///
/// Sets `AssemblyState.a3d_handle` to trigger the assembly system.
/// TODO(Phase 7): Replace DEFAULT_SCENE_PATH with GameConfig resource.
pub fn load_a3d_scene(mut assembly: ResMut<AssemblyState>, asset_server: Res<AssetServer>) {
    let scene_path = std::env::var("A3D_MAP")
        .unwrap_or_else(|_| "game_map_y8.a3d".to_string());
    assembly.a3d_handle = Some(asset_server.load(&scene_path));
    info!("load_a3d_scene: loading '{}'", scene_path);
}

/// Assembly system that builds runtime structures from loaded A3D assets.
///
/// Runs each frame until assembly completes. Guards:
/// 1. Already assembled -> return
/// 2. No handle set -> return
/// 3. Asset not yet loaded -> return
/// 4. Sub-assets not ready -> return
///
/// When all sub-assets are ready, builds RuntimeTerrain, RuntimeWorld,
/// inserts RuntimeMaterials Resource, and queues AKM mesh loads.
#[allow(clippy::too_many_arguments)]
pub fn a3d_assembly_system(
    mut commands: Commands,
    mut assembly: ResMut<AssemblyState>,
    a3d_files: Res<Assets<A3dFile>>,
    terrains: Res<Assets<A3dTerrain>>,
    worlds: Res<Assets<A3dWorld>>,
    mat_tables: Res<Assets<MaterialTable>>,
    mut runtime_terrain: ResMut<RuntimeTerrain>,
    mut runtime_world: ResMut<RuntimeWorld>,
    asset_server: Res<AssetServer>,
    _akm_assets: Res<Assets<AkmMesh>>,
    mut mesh_registry: ResMut<MeshRegistry>,
) {
    // Guard 1: already assembled
    if assembly.assembled {
        return;
    }

    // Guard 2: no handle set
    let Some(handle) = assembly.a3d_handle.as_ref() else {
        return;
    };

    // Guard 3: A3dFile not loaded yet
    let Some(a3d_file) = a3d_files.get(handle) else {
        return;
    };

    // Guard 4: sub-assets not loaded yet
    let Some(terrain) = terrains.get(&a3d_file.terrain) else {
        return;
    };
    let Some(mat_table) = mat_tables.get(&a3d_file.materials) else {
        return;
    };
    let Some(world) = worlds.get(&a3d_file.world) else {
        return;
    };

    // (a) Build RuntimeTerrain
    let built_terrain = RuntimeTerrain::build_from_parsed(terrain);
    let patch_count = built_terrain.patch_count;
    *runtime_terrain = built_terrain;

    // (a.2) Compute terrain shadows (load-time precomputation, NOT per-frame).
    // P5-004 FIX: Shadow call site added by Plan 05-06.
    // P5-128 FIX: Inserted AFTER build_from_parsed, BEFORE RuntimeMaterials insert.
    update_terrain_dark(&mut runtime_terrain, default_light_dir());
    info!(
        "Terrain shadow computation complete ({} patches)",
        patch_count
    );

    // (b) Build RuntimeWorld
    let built_world = RuntimeWorld::build_from_parsed(world);
    let instance_count = built_world.instances.len();
    *runtime_world = built_world;

    // (c) Extract MaterialTable and insert as standalone Resource
    let materials = convert_material_table(mat_table);
    let material_count = materials.len();
    commands.insert_resource(RuntimeMaterials(materials));

    // (d) Queue AKM mesh loads for mesh instances
    for inst in &runtime_world.instances {
        if let crate::world::instance::RuntimeInstance::Mesh { mesh_id, .. } = inst {
            mesh_registry
                .meshes
                .entry(mesh_id.clone())
                .or_insert_with(|| asset_server.load(format!("meshes/{mesh_id}")));
        }
    }

    // (e) Mark assembly complete
    assembly.assembled = true;

    info!(
        "A3D assembly complete: {} patches, {} instances, {} materials, {} unique meshes queued",
        patch_count,
        instance_count,
        material_count,
        mesh_registry.meshes.len()
    );
}

/// System that polls pending AKM mesh handles and transfers loaded data
/// into `MeshRegistry.loaded` for the render pipeline to consume.
///
/// Runs each frame until all meshes are loaded. Once `meshes` (pending) is
/// empty, all queued meshes have been transferred to `loaded`.
pub fn poll_akm_meshes(
    mut mesh_registry: ResMut<MeshRegistry>,
    akm_assets: Res<Assets<AkmMesh>>,
) {
    if mesh_registry.meshes.is_empty() {
        return;
    }

    let mut newly_loaded = Vec::new();
    for (name, handle) in mesh_registry.meshes.iter() {
        if let Some(mesh) = akm_assets.get(handle) {
            newly_loaded.push((name.clone(), mesh.clone()));
        }
    }

    for (name, mesh) in newly_loaded {
        mesh_registry.meshes.remove(&name);
        let verts = mesh.vertices.len();
        let faces = mesh.faces.len();
        mesh_registry.loaded.insert(name.clone(), mesh);
        info!(
            "AKM mesh loaded: '{}' ({} vertices, {} faces)",
            name, verts, faces,
        );
    }
}

// ---------------------------------------------------------------------------
// Material conversion
// ---------------------------------------------------------------------------

/// Convert a single asset MatCell to a render MatCell via field-by-field copy.
///
/// The two MatCell types are distinct Rust types (asset_loader vs render::material)
/// with identical layouts. Cannot transmute -- explicit field copy.
fn convert_matcell(src: &AssetMatCell) -> RenderMatCell {
    RenderMatCell {
        fg: src.fg,
        gl: src.gl,
        bg: src.bg,
        flags: src.flags,
    }
}

/// Convert the parsed MaterialTable (from A3D asset) to a Vec<Material> for rendering.
///
/// The MaterialTable must contain exactly 256 materials, each with 4 elevations x 16 diffuse levels.
pub fn convert_material_table(table: &MaterialTable) -> Vec<Material> {
    assert_eq!(
        table.materials.len(),
        256,
        "MaterialTable must contain exactly 256 materials, got {}",
        table.materials.len()
    );

    table
        .materials
        .iter()
        .map(|elevations| {
            let mut shade = [[RenderMatCell::default(); 16]; 4];
            for (elv_idx, elv_row) in elevations.iter().enumerate() {
                for (dif_idx, asset_mc) in elv_row.iter().enumerate() {
                    shade[elv_idx][dif_idx] = convert_matcell(asset_mc);
                }
            }
            Material { shade, mode: 0 }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_loader::a3d_terrain::MatCell as AssetMatCell;

    #[test]
    fn test_convert_material_table() {
        // Create a minimal 256-entry MaterialTable
        let mut materials = Vec::with_capacity(256);
        for mat_idx in 0..256u16 {
            let mut elevations = [[AssetMatCell::default(); 16]; 4];
            // Set a known value at elevation 0, diffuse level 0
            elevations[0][0] = AssetMatCell {
                fg: [mat_idx as u8, 0, 0],
                gl: b'.',
                bg: [0, mat_idx as u8, 0],
                flags: 0,
            };
            materials.push(elevations);
        }
        let table = MaterialTable { materials };

        let result = convert_material_table(&table);
        assert_eq!(result.len(), 256);

        // Verify first material's shade[0][0]
        assert_eq!(result[0].shade[0][0].fg, [0, 0, 0]);
        assert_eq!(result[0].shade[0][0].gl, b'.');

        // Verify material 100's shade[0][0].fg[0] == 100
        assert_eq!(result[100].shade[0][0].fg[0], 100);
        assert_eq!(result[100].shade[0][0].bg[1], 100);

        // Verify mode is 0
        for mat in &result {
            assert_eq!(mat.mode, 0);
        }
    }

    #[test]
    fn test_assembly_state_default() {
        let state = AssemblyState::default();
        assert!(!state.assembled);
        assert!(state.a3d_handle.is_none());
    }

    #[test]
    fn test_matcell_layout_equivalence() {
        // Verify both MatCell types are 8 bytes
        assert_eq!(std::mem::size_of::<AssetMatCell>(), 8);
        assert_eq!(std::mem::size_of::<RenderMatCell>(), 8);

        // Field-by-field conversion preserves values
        let src = AssetMatCell {
            fg: [100, 150, 200],
            gl: 65,
            bg: [10, 20, 30],
            flags: 0x0F,
        };
        let dst = convert_matcell(&src);
        assert_eq!(dst.fg, src.fg);
        assert_eq!(dst.gl, src.gl);
        assert_eq!(dst.bg, src.bg);
        assert_eq!(dst.flags, src.flags);
    }

    #[test]
    fn test_mesh_registry_default_empty() {
        let reg = MeshRegistry::default();
        assert!(reg.meshes.is_empty());
        assert!(reg.loaded.is_empty());
    }

    /// F235 investigation: verify that the real game_map_y8.a3d material table
    /// has non-zero glyphs at elevation 3, mid-diffuse (index 8).
    ///
    /// The terrain visual indices observed in the map are 0, 1, 5, 15 (no bit-15
    /// flags). When no bit-15 flag is set the C++ resolve code uses elevation=3
    /// for all cells. If shade[3][8].gl == 0 those cells would render as blank.
    #[test]
    #[ignore] // requires real asset: cargo test test_real_material_elevation_3 -- --ignored --nocapture
    fn test_real_material_elevation_3() {
        use crate::asset_loader::a3d_terrain::{parse_material_section, parse_terrain_section};

        let a3d_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("game_map_y8.a3d");
        let data = std::fs::read(&a3d_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {e}", a3d_path.display()));

        // Parse terrain section to find where material table starts.
        let (_terrain, terrain_bytes) =
            parse_terrain_section(&data).expect("Failed to parse terrain section");

        // Parse material table from the bytes after the terrain section.
        let (mat_table, _mat_bytes) = parse_material_section(&data[terrain_bytes..])
            .expect("Failed to parse material section");

        let materials = convert_material_table(&mat_table);

        let target_indices: &[usize] = &[0, 1, 5, 15];
        let mid_diffuse: usize = 8; // mid-range diffuse level

        for &mat_idx in target_indices {
            println!("--- Material {mat_idx} ---");
            for elv in 0..4usize {
                let cell = &materials[mat_idx].shade[elv][mid_diffuse];
                println!(
                    "  shade[{elv}][{mid_diffuse}]: gl={:3} (0x{:02x}, '{}')  fg=[{},{},{}]  bg=[{},{},{}]  flags=0x{:02x}",
                    cell.gl,
                    cell.gl,
                    if cell.gl >= 0x20 && cell.gl < 0x7f {
                        cell.gl as char
                    } else {
                        '.'
                    },
                    cell.fg[0], cell.fg[1], cell.fg[2],
                    cell.bg[0], cell.bg[1], cell.bg[2],
                    cell.flags,
                );
            }
        }

        // Assert elevation 3, mid-diffuse has a non-zero glyph for each target material.
        for &mat_idx in target_indices {
            let cell = &materials[mat_idx].shade[3][mid_diffuse];
            assert!(
                cell.gl != 0,
                "Material {mat_idx}: shade[3][{mid_diffuse}].gl is 0 (empty glyph) — \
                 terrain cells using this material at elevation 3 would render blank!"
            );
        }
    }
}
