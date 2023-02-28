struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
};

// Window physical width and height
const WW: f32 = 3840.0;
const WH: f32 = 2160.0;

// Font width and height
const FW: f32 = 4.0;
const FH: f32 = 6.0;

@group(1) @binding(0) var<storage, read> text_data: array<u32>;

// Top left coord

@vertex
fn vs_main(
    @builtin(vertex_index) ii: u32,
    @builtin(instance_index) instance: u32,
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
    let WS = FW / WW * 2.0;
    let HS = FH / WH * 2.0;

    let sx = 0.0;
    let sy = FH * f32(instance + 1u);

    // Create the vertex
    out.clip_position = vec4(
        -1.0 + f32(div6) * WS + x * WS + sx * (2.0 / WW),
        -1.0 + y * HS + (WH - sy) * (2.0 / WH),
        0.0,
        1.0,
    );

    // Compute texture coord
    let char_and_color = text_data[div6];
    let ch = char_and_color & 0xffu;
    out.tex_coords = vec2(
        (1.0 / 16.0) * (f32(ch &  0xfu) + x),
        (1.0 / 16.0) * (f32(ch >> 0x4u) - y + 1.0),
    );

    out.color = vec4(
        f32((char_and_color >>  8u) & 0xffu) / 255.0,
        f32((char_and_color >> 16u) & 0xffu) / 255.0,
        f32((char_and_color >> 24u) & 0xffu) / 255.0,
        1.0
    );

    return out;
}

// All our fonts
@group(0) @binding(0)  var font_4x6:   texture_2d<f32>;
@group(0) @binding(1)  var font_6x8:   texture_2d<f32>;
@group(0) @binding(2)  var font_6x9:   texture_2d<f32>;
@group(0) @binding(3)  var font_6x10:  texture_2d<f32>;
@group(0) @binding(4)  var font_8x12:  texture_2d<f32>;
@group(0) @binding(5)  var font_8x14:  texture_2d<f32>;
@group(0) @binding(6)  var font_8x15:  texture_2d<f32>;
@group(0) @binding(7)  var font_8x16:  texture_2d<f32>;
@group(0) @binding(8)  var font_12x20: texture_2d<f32>;
@group(0) @binding(9)  var font_16x24: texture_2d<f32>;
@group(0) @binding(10) var font_20x32: texture_2d<f32>;
@group(0) @binding(11) var font_24x36: texture_2d<f32>;

// Texture sampler for our fonts
@group(0) @binding(12) var font_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(font_4x6, font_sampler, in.tex_coords) * in.color;
}

