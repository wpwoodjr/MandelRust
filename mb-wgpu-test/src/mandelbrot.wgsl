struct Uniforms {
    xmin: f32,
    ymin: f32,
    xmax: f32,
    ymax: f32,
    max_iterations: u32,
};

struct Output {
    data: array<u32>,
};

@binding(0) @group(0)
var<storage, read_write> output: Output;

@binding(1) @group(0)
var<uniform> uniforms: Uniforms;

fn mandelbrot(c: vec2<f32>, max_iterations: u32) -> u32 {
    var z = vec2<f32>(0.0, 0.0);
    var i: u32 = 0u;
    for (; i < max_iterations; i = i + 1u) {
        if (dot(z, z) > 4.0) {
            break;
        }
        z = vec2<f32>(z.x * z.x - z.y * z.y, 2.0 * z.x * z.y) + c;
    }
    return i;
}

@compute
@workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;

    let c = vec2<f32>(uniforms.xmin, uniforms.ymax) + (vec2<f32>(uniforms.xmax - uniforms.xmin, uniforms.ymin - uniforms.ymax) * (vec2<f32>(f32(x), f32(y)) / vec2<f32>(f32(1600), f32(1200))));
    var color = mandelbrot(c, uniforms.max_iterations);
    let index = x + y * u32(1600);

    if (color == uniforms.max_iterations) {
        color = 0xFFu;
    }
    output.data[index] = color | (color << 8u) | (color << 16u) | (0xFFu << 24u);
}
