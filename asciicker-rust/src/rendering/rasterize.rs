// Triangle rasterization
use super::buffer::{SampleBuffer, Sample};

pub fn rasterize_triangle(
    buffer: &mut SampleBuffer,
    v0: [f32; 2],
    v1: [f32; 2],
    v2: [f32; 2],
    color: u16,
) {
    // Edge function rasterization
    let area = edge_function(v0, v1, v2);
    if area <= 0.0 { return; }
    
    let min_x = v0[0].min(v1[0]).min(v2[0]).max(0.0) as i32;
    let max_x = v0[0].max(v1[0]).max(v2[0]).min(buffer.width as f32) as i32;
    let min_y = v0[1].min(v1[1]).min(v2[1]).max(0.0) as i32;
    let max_y = v0[1].max(v1[1]).max(v2[1]).min(buffer.height as f32) as i32;
    
    for y in min_y..max_y {
        for x in min_x..max_x {
            let p = [x as f32 + 0.5, y as f32 + 0.5];
            let w0 = edge_function(v1, v2, p);
            let w1 = edge_function(v2, v0, p);
            let w2 = edge_function(v0, v1, p);
            
            if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                let idx = (y * buffer.width + x) as usize;
                if idx < buffer.samples.len() {
                    buffer.samples[idx].visual = color;
                }
            }
        }
    }
}

fn edge_function(a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> f32 {
    (c[0] - a[0]) * (b[1] - a[1]) - (c[1] - a[1]) * (b[0] - a[0])
}
