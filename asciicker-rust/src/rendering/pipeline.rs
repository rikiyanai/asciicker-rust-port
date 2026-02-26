// Rendering pipeline - 6-stage pipeline

/// Render pipeline stages
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RenderPhase {
    Clear,
    Terrain,
    World,
    Shadow,
    Reflection,
    Resolve,
    Sprites,
    UI,
}
