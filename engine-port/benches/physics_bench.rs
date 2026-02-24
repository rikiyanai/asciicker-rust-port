use criterion::{Criterion, criterion_group, criterion_main};

fn bench_collision_sweep(c: &mut Criterion) {
    c.bench_function("collision_sweep_1_entity_50_tris", |b| {
        use asciicker_engine::physics::soup::SoupItem;
        // Build 50 triangles (realistic terrain patch)
        let soup: Vec<SoupItem> = (0..50)
            .map(|i| {
                let offset = i as f32 * 0.1;
                SoupItem {
                    tri: [
                        [offset, 0.0, 0.0],
                        [offset + 1.0, 0.0, 0.0],
                        [offset + 0.5, 1.0, 0.0],
                    ],
                    material: 0,
                    nrm: [0.0, 0.0, 1.0, 0.0],
                }
            })
            .collect();
        let pos = [0.0f32, 0.0, 1.5];
        let vel = [0.0f32, 0.0, -1.0];
        b.iter(|| {
            for item in &soup {
                let _ =
                    criterion::black_box(asciicker_engine::physics::collision::check_collision(
                        &item.tri, &item.nrm, &pos, &vel,
                    ));
            }
        });
    });
}

fn bench_forces_accumulation(c: &mut Criterion) {
    c.bench_function("forces_accumulation", |b| {
        use asciicker_engine::physics::{PhysicsIO, PhysicsState, forces::accumulate_forces};
        let dt = 1.0_f32 / 66.667_f32;
        let mut physics_state = PhysicsState::default();
        let physics_io = PhysicsIO {
            water: f32::NEG_INFINITY,
            ..Default::default()
        };
        // accumulate_forces signature: (state: &mut PhysicsState, io: &PhysicsIO, dt: f32)
        // io is immutable -- only reads x_force, y_force, yaw
        b.iter(|| {
            criterion::black_box(accumulate_forces(&mut physics_state, &physics_io, dt));
        });
    });
}

fn bench_full_physics_frame(c: &mut Criterion) {
    c.bench_function("full_physics_frame", |b| {
        use asciicker_engine::physics::{
            PhysicsIO, PhysicsState,
            collision::check_collision,
            forces::{accumulate_forces, update_grounded},
            soup::SoupItem,
        };
        let dt = 1.0_f32 / 66.667_f32;
        // 50 triangles (realistic terrain patch around entity)
        let soup: Vec<SoupItem> = (0..50)
            .map(|i| {
                let offset = i as f32 * 0.1;
                SoupItem {
                    tri: [
                        [offset, 0.0, 0.0],
                        [offset + 1.0, 0.0, 0.0],
                        [offset + 0.5, 1.0, 0.0],
                    ],
                    material: 0,
                    nrm: [0.0, 0.0, 1.0, 0.0],
                }
            })
            .collect();
        let mut physics_state = PhysicsState::default();
        let physics_io = PhysicsIO {
            water: f32::NEG_INFINITY,
            ..Default::default()
        };
        b.iter(|| {
            criterion::black_box(accumulate_forces(&mut physics_state, &physics_io, dt));
            for item in &soup {
                let _ = criterion::black_box(check_collision(
                    &item.tri,
                    &item.nrm,
                    &physics_io.pos,
                    &physics_state.vel(),
                ));
            }
            criterion::black_box(update_grounded(&mut physics_state, dt));
        });
    });
}

criterion_group!(
    benches,
    bench_collision_sweep,
    bench_forces_accumulation,
    bench_full_physics_frame
);
criterion_main!(benches);
