//! Sphere-triangle collision detection.
//!
//! Port of C++ `CheckCollision` (physics.cpp:461-824). Implements the 3-test
//! cascade: face intersection with barycentric containment, then vertex
//! collision (sphere-vs-point), then edge collision (sphere-vs-segment).
//!
//! TRAP-P01: No magic 2.0 sentinel. Uses `CollisionResult::Miss` instead.

/// Result of a sphere-triangle collision test.
///
/// TRAP-P01: Replaces C++ convention of returning 2.0 for "no collision".
#[derive(Debug, Clone)]
pub enum CollisionResult {
    /// Collision detected at time-of-impact `toi` in `[0, 1]` with
    /// contact point on the triangle surface.
    Hit { toi: f32, contact: [f32; 3] },
    /// No collision detected within the current timestep.
    Miss,
}

fn dot3(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross3(a: &[f32; 3], b: &[f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot4(nrm: &[f32; 4], p: &[f32; 3]) -> f32 {
    nrm[0] * p[0] + nrm[1] * p[1] + nrm[2] * p[2]
}

/// Sphere-triangle collision test using the 3-test cascade from C++ physics.cpp.
///
/// The sphere has unit radius in the coordinate space of `tri` and `nrm`.
/// Callers must transform geometry to sphere space before calling.
///
/// # Arguments
/// * `tri` - Triangle vertices `[[x,y,z]; 3]` in sphere space
/// * `nrm` - Plane equation `[nx, ny, nz, w]` (unit normal + plane offset)
/// * `pos` - Sphere center position in sphere space
/// * `vel` - Sphere velocity in sphere space
///
/// # Returns
/// `CollisionResult::Hit` with TOI in [0, 1] and contact point, or `Miss`.
///
/// # Algorithm
/// 1. **Face test:** Plane intersection + barycentric containment
/// 2. **Vertex test:** Sphere-vs-point for each of the 3 vertices
/// 3. **Edge test:** Sphere-vs-segment for each of the 3 edges
///
/// R19-M01: Vertex tested before edge, matching C++ source order (physics.cpp:667-817).
pub fn check_collision(
    tri: &[[f32; 3]; 3],
    nrm: &[f32; 4],
    pos: &[f32; 3],
    vel: &[f32; 3],
) -> CollisionResult {
    // Point on sphere surface closest to plane at t=0
    let col = [pos[0] - nrm[0], pos[1] - nrm[1], pos[2] - nrm[2]];

    let vel_dot_nrm = -(vel[0] * nrm[0] + vel[1] * nrm[1] + vel[2] * nrm[2]);

    if vel_dot_nrm <= 0.0 {
        // Backface or parallel: velocity not approaching plane
        return CollisionResult::Miss;
    }

    let dist = dot4(nrm, &col) + nrm[3];
    let mut contact = [0.0f32; 3];

    let plane_t;

    if dist > 0.0 {
        // Normal hit: sphere in front of plane
        plane_t = dist / vel_dot_nrm;
    } else if dist > -1.0 {
        // Embedded case: sphere has slightly penetrated
        let pen = 1.0 + dist;
        contact[0] = col[0] - pen * nrm[0];
        contact[1] = col[1] - pen * nrm[1];
        contact[2] = col[2] - pen * nrm[2];
        plane_t = 0.0;
    } else {
        // Deeply embedded (dist <= -1): ignore to prevent explosion
        return CollisionResult::Miss;
    }

    // Contact point at plane_t (overwritten for embedded case above, but
    // re-computed here matching C++ flow where both branches set contact_pos)
    contact[0] = col[0] + plane_t * vel[0];
    contact[1] = col[1] + plane_t * vel[1];
    contact[2] = col[2] + plane_t * vel[2];

    // Barycentric containment test
    let edge = [
        [
            tri[1][0] - tri[0][0],
            tri[1][1] - tri[0][1],
            tri[1][2] - tri[0][2],
        ],
        [
            tri[2][0] - tri[1][0],
            tri[2][1] - tri[1][1],
            tri[2][2] - tri[1][2],
        ],
        [
            tri[0][0] - tri[2][0],
            tri[0][1] - tri[2][1],
            tri[0][2] - tri[2][2],
        ],
    ];

    let vect = [
        [
            contact[0] - tri[0][0],
            contact[1] - tri[0][1],
            contact[2] - tri[0][2],
        ],
        [
            contact[0] - tri[1][0],
            contact[1] - tri[1][1],
            contact[2] - tri[1][2],
        ],
        [
            contact[0] - tri[2][0],
            contact[1] - tri[2][1],
            contact[2] - tri[2][2],
        ],
    ];

    let c0 = cross3(&edge[0], &vect[0]);
    let d0 = dot4(nrm, &c0);

    let c1 = cross3(&edge[1], &vect[1]);
    let d1 = dot4(nrm, &c1);

    let c2 = cross3(&edge[2], &vect[2]);
    let d2 = dot4(nrm, &c2);

    if d0 >= 0.0 && d1 >= 0.0 && d2 >= 0.0 {
        // Face hit: contact inside triangle
        if plane_t > 1.0 {
            return CollisionResult::Miss;
        }
        return CollisionResult::Hit {
            toi: plane_t,
            contact,
        };
    }

    // Face test failed: try vertex then edge (R19-M01: vertex before edge)
    let mut best_t = 2.0f32;
    let mut best_contact = [0.0f32; 3];

    // --- Vertex collision test ---
    let a_coeff = dot3(vel, vel);
    if a_coeff > 0.0 {
        for vertex in tri {
            let p_ps = [pos[0] - vertex[0], pos[1] - vertex[1], pos[2] - vertex[2]];

            let b_coeff = 2.0 * dot3(&p_ps, vel);
            let c_coeff = dot3(&p_ps, &p_ps) - 1.0;

            let discriminant = b_coeff * b_coeff - 4.0 * a_coeff * c_coeff;
            if discriminant >= 0.0 {
                let t = (-b_coeff - discriminant.sqrt()) / (2.0 * a_coeff);
                if (0.0..=1.0).contains(&t) && t < best_t {
                    best_t = t;
                    best_contact = *vertex;
                }
            }
        }
    }

    // --- Edge collision test ---
    for c in 0..3 {
        let vcvc = dot3(&edge[c], &edge[c]);
        if vcvc < 1e-12 {
            continue; // Degenerate edge
        }

        let p_pc = [pos[0] - tri[c][0], pos[1] - tri[c][1], pos[2] - tri[c][2]];

        let vc_dot_p_pc = dot3(&edge[c], &p_pc);

        let u_vec = [
            p_pc[0] * vcvc - edge[c][0] * vc_dot_p_pc,
            p_pc[1] * vcvc - edge[c][1] * vc_dot_p_pc,
            p_pc[2] * vcvc - edge[c][2] * vc_dot_p_pc,
        ];

        let vc_dot_v = dot3(&edge[c], vel);

        let v_vec = [
            vel[0] * vcvc - edge[c][0] * vc_dot_v,
            vel[1] * vcvc - edge[c][1] * vc_dot_v,
            vel[2] * vcvc - edge[c][2] * vc_dot_v,
        ];

        let a_edge = dot3(&v_vec, &v_vec);
        let b_edge = 2.0 * dot3(&u_vec, &v_vec);
        let c_edge = dot3(&u_vec, &u_vec) - vcvc * vcvc;

        let discriminant = b_edge * b_edge - 4.0 * a_edge * c_edge;
        if discriminant >= 0.0 {
            let t = (-b_edge - discriminant.sqrt()) / (2.0 * a_edge);
            if (0.0..=1.0).contains(&t) && t < best_t {
                // Check h parameter: is contact on the segment [0, 1]?
                let at_t = [
                    pos[0] + t * vel[0] - tri[c][0],
                    pos[1] + t * vel[1] - tri[c][1],
                    pos[2] + t * vel[2] - tri[c][2],
                ];
                let h_mul_vc = dot3(&at_t, &edge[c]);
                if h_mul_vc >= 0.0 && h_mul_vc <= vcvc {
                    best_t = t;
                    let h_div_vc = h_mul_vc / vcvc;
                    best_contact = [
                        tri[c][0] + edge[c][0] * h_div_vc,
                        tri[c][1] + edge[c][1] * h_div_vc,
                        tri[c][2] + edge[c][2] * h_div_vc,
                    ];
                }
            }
        }
    }

    if best_t <= 1.0 {
        CollisionResult::Hit {
            toi: best_t,
            contact: best_contact,
        }
    } else {
        CollisionResult::Miss
    }
}

/// Ray-triangle intersection using the Möller-Trumbore algorithm.
///
/// Returns Option<toi> if the ray intersects the triangle within [0, max_dist].
pub fn ray_triangle_intersection(
    origin: &[f32; 3],
    dir: &[f32; 3],
    tri: &[[f32; 3]; 3],
    max_dist: f32,
) -> Option<f32> {
    const EPSILON: f32 = 1e-6;

    let edge1 = [
        tri[1][0] - tri[0][0],
        tri[1][1] - tri[0][1],
        tri[1][2] - tri[0][2],
    ];
    let edge2 = [
        tri[2][0] - tri[0][0],
        tri[2][1] - tri[0][1],
        tri[2][2] - tri[0][2],
    ];

    let h = cross3(dir, &edge2);
    let a = dot3(&edge1, &h);

    if a > -EPSILON && a < EPSILON {
        return None; // Parallel
    }

    let f = 1.0 / a;
    let s = [
        origin[0] - tri[0][0],
        origin[1] - tri[0][1],
        origin[2] - tri[0][2],
    ];
    let u = f * dot3(&s, &h);

    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let q = cross3(&s, &edge1);
    let v = f * dot3(dir, &q);

    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = f * dot3(&edge2, &q);

    if t > EPSILON && t < max_dist {
        Some(t)
    } else {
        None
    }
}
