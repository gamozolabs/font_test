//! Preload all fonts into the GPU textures

use std::num::NonZeroU64;
use wgpu::util::DeviceExt;
use image::GenericImageView;
use crate::Castable;

/// An instance of a font
pub struct Font {
    /// Bind group for this font
    pub bind_group: wgpu::BindGroup,

    /// Width of a character
    pub width: u32,

    /// Height of a character
    pub height: u32,
}

/// Loaded fonts
pub struct Fonts {
    /// Bind group layout for a font
    ///
    /// Contains the font texture at 0, and the sampler at 1
    pub bind_group_layout: wgpu::BindGroupLayout,

    /// All loaded fonts (in order of bindings)
    pub fonts: Vec<Font>,
}

#[derive(Clone, Copy)]
#[repr(usize)]
pub enum FontSize {
    Size4x6,
    Size6x8,
    Size6x10,
    Size8x12,
    Size8x14,
    Size8x15,
    Size8x16,
    Size12x20,
    Size16x24,
    Size24x36,
}

/// Load all fonts in our database into the `device`
pub fn load_fonts(device: &wgpu::Device, queue: &wgpu::Queue) -> Fonts {
    // All fonts
    let font_data = [
        include_bytes!("../fonts/4x6.png").as_slice(),
        include_bytes!("../fonts/6x8.png").as_slice(),
        include_bytes!("../fonts/6x10.png").as_slice(),
        include_bytes!("../fonts/8x12.png").as_slice(),
        include_bytes!("../fonts/8x14.png").as_slice(),
        include_bytes!("../fonts/8x15.png").as_slice(),
        include_bytes!("../fonts/8x16.png").as_slice(),
        include_bytes!("../fonts/12x20.png").as_slice(),
        include_bytes!("../fonts/16x24.png").as_slice(),
        include_bytes!("../fonts/24x36.png").as_slice(),
    ];

    // Create the sampler
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter:     wgpu::FilterMode::Linear,
            min_filter:     wgpu::FilterMode::Nearest,
            mipmap_filter:  wgpu::FilterMode::Nearest,
            ..Default::default()
        });

    // Construct the bind group layout for all fonts
    let bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding:    0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    count:      None,
                    ty: wgpu::BindingType::Texture {
                        multisampled:   false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type:    wgpu::TextureSampleType::Float {
                            filterable: true
                        },
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding:    1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    count:      None,
                    ty: wgpu::BindingType::Sampler(
                        wgpu::SamplerBindingType::Filtering),
                },
                wgpu::BindGroupLayoutEntry {
                    binding:    2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    count:      None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(8),
                    },
                },
            ],
            label: None,
        });

    // Create font database
    let mut fonts = Vec::new();

    // Load every font
    for bytes in font_data {
        // Load the font using the `image` crate
        let image = image::load_from_memory(bytes)
            .expect("Failed to load font");

        // Convert font to RGBA8 format, which is what we use for our texture
        let rgba = image.to_rgba8();

        // Get the dimensions of the font
        let dimensions = image.dimensions();

        // Make sure the file format is good
        assert!(dimensions.0 % 16 == 0 && dimensions.1 % 16 == 0,
            "Yucky font file format");

        // Create a new texture capable of holding the font bitmap
        let texture = device.create_texture_with_data(
            &queue,
            &wgpu::TextureDescriptor {
                size: wgpu::Extent3d {
                    width:                 dimensions.0,
                    height:                dimensions.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count:    1,
                dimension:       wgpu::TextureDimension::D2,
                format:          wgpu::TextureFormat::Rgba8UnormSrgb,
                usage:           wgpu::TextureUsages::TEXTURE_BINDING,
                label:           None,
                view_formats:    &[],
            },
            &rgba,
        );

        // Get a view of the texture
        let texture_view = texture.create_view(
            &wgpu::TextureViewDescriptor::default());

        // Create a buffer for the uniform which holds the text size
        let buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                usage: wgpu::BufferUsages::UNIFORM,
                contents: (
                    (dimensions.0 / 16) as f32,
                    (dimensions.1 / 16) as f32,
                ).cast(),
            });

        // Save the font info
        fonts.push(Font {
            width:      dimensions.0 / 16,
            height:     dimensions.0 / 16,
            bind_group: device.create_bind_group(
                &wgpu::BindGroupDescriptor {
                    layout:  &bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding:  0,
                            resource: wgpu::BindingResource::TextureView(
                                &texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding:  1,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer: &buffer,
                                offset: 0,
                                size:   NonZeroU64::new(8),
                            }),
                        },
                    ],
                    label: None,
                }),
        });
    }

    Fonts {
        fonts,
        bind_group_layout,
    }
}

