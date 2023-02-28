struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0)       tex_coords:    vec2<f32>,
    @location(1)       color:         vec4<f32>,
};

struct Globals {
    win_width:  f32,
    win_height: f32,
};

@group(1) @binding(0) var<storage, read> text_data: array<u32>;
@group(1) @binding(1) var<uniform> globals: Globals;

// Font size
@group(0) @binding(2) var<uniform> font_size: vec2<f32>;

struct PushConstants {
    rgba:   vec4<f32>,
    xy:     vec2<f32>,
    offset: u32,
}

var<push_constant> pc: PushConstants;

@vertex
fn vs_main(
    @builtin(vertex_index) ii: u32,
) -> VertexOutput {
    var out: VertexOutput;

    // Always correct while ii in in the range [0, 32766)
    // 0 (0.0, 0.0)
    // 1 (1.0, 1.0)
    // 2 (0.0, 1.0)
    // 3 (1.0, 1.0)
    // 4 (0.0, 0.0)
    // 5 (1.0, 0.0)

    // Compute div3 and div6
    let div3 = ((ii + 2u) * 0x5556u) >> 16u;
    let div6 = ((ii     ) * 0x5556u) >> 17u;

    // Generate X and Y coords, where (1, 1) is top right,
    // (0, 0) is bottom left
    let x = f32(ii   & 1u);
    let y = f32(div3 & 1u);

    // Figure out scaling
    let WS = font_size.x / globals.win_width  * 2.0;
    let HS = font_size.y / globals.win_height * 2.0;

    // Create the vertex
    out.clip_position = vec4(
        -1.0 + f32(div6) * WS + x * WS + pc.xy.x * (2.0 / globals.win_width),
        -1.0 + y * HS + (globals.win_height - pc.xy.y) * (2.0 / globals.win_height),
        0.0,
        1.0,
    );

    // Compute texture coord
    let char_and_color = text_data[pc.offset + (div6 >> 2u)];
    let ch = (char_and_color >> ((div6 & 3u) * 8u)) & 0xffu;
    out.tex_coords = vec2(
        (1.0 / 16.0) * (f32(ch &  0xfu) + x),
        (1.0 / 16.0) * (f32(ch >> 0x4u) - y + 1.0),
    );

    out.color = pc.rgba;

    return out;
}

// All our fonts
@group(0) @binding(0) var font_atlas: texture_2d<f32>;

// Texture sampler for our fonts
@group(0) @binding(1) var font_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(font_atlas, font_sampler, in.tex_coords) * in.color;
}

