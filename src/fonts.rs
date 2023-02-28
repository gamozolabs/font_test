//! Preload all fonts into the GPU textures

use wgpu::util::DeviceExt;
use image::GenericImageView;

/// An instance of a font
pub(super) struct Font {
    /// Loaded texture
    texture: wgpu::TextureView,

    /// Width of a character
    pub(super) width: u32,

    /// Height of a character
    pub(super) height: u32,
}

/// Loaded fonts
pub(super) struct Fonts {
    /// Bind group layout
    /// Bindings are all fonts from `0..fonts.len()`, and then the
    /// `fonts.len()`th index is the texture sampler
    pub(super) bind_group_layout: wgpu::BindGroupLayout,

    /// Bind group
    pub(super) bind_group: wgpu::BindGroup,

    /// All loaded fonts (in order of bindings)
    pub(super) fonts: Vec<Font>,
}

/// Load all fonts in our database into the `device`
pub(super) fn load_fonts(device: &wgpu::Device, queue: &wgpu::Queue) -> Fonts {
    // All fonts
    let font_data = [
        include_bytes!("../fonts/4x6.png").as_slice(),
        include_bytes!("../fonts/6x8.png").as_slice(),
        include_bytes!("../fonts/6x9.png").as_slice(),
        include_bytes!("../fonts/6x10.png").as_slice(),
        include_bytes!("../fonts/8x12.png").as_slice(),
        include_bytes!("../fonts/8x14.png").as_slice(),
        include_bytes!("../fonts/8x15.png").as_slice(),
        include_bytes!("../fonts/8x16.png").as_slice(),
        include_bytes!("../fonts/12x20.png").as_slice(),
        include_bytes!("../fonts/16x24.png").as_slice(),
        include_bytes!("../fonts/20x32.png").as_slice(),
        include_bytes!("../fonts/24x36.png").as_slice(),
    ];

    // Create font database
    let mut fonts = Vec::new();

    // Load every font
    let mut bind_group_entries        = Vec::new();
    let mut bind_group_layout_entries = Vec::new();
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

        // Save the font info
        fonts.push(Font {
            texture: texture_view,
            width:   dimensions.0 / 16,
            height:  dimensions.0 / 16,
        });
    }

    // Create the bind group info
    for (ii, font) in fonts.iter().enumerate() {
        // Add layout for this font
        bind_group_layout_entries.push(
            wgpu::BindGroupLayoutEntry {
                binding:    ii as u32,
                visibility: wgpu::ShaderStages::FRAGMENT,
                count:      None,
                ty: wgpu::BindingType::Texture {
                    multisampled:   false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type:    wgpu::TextureSampleType::Float {
                        filterable: true
                    },
                },
            });
        bind_group_entries.push(
            wgpu::BindGroupEntry {
                binding:  ii as u32,
                resource: wgpu::BindingResource::TextureView(
                    &font.texture),
            });
    }

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

    // Add the layout for the sampler
    bind_group_layout_entries.push(
        wgpu::BindGroupLayoutEntry {
            binding:    fonts.len() as u32,
            visibility: wgpu::ShaderStages::FRAGMENT,
            count:      None,
            ty: wgpu::BindingType::Sampler(
                wgpu::SamplerBindingType::Filtering),
        });

    // Add the sampler to the bind group
    bind_group_entries.push(
        wgpu::BindGroupEntry {
            binding:  fonts.len() as u32,
            resource: wgpu::BindingResource::Sampler(&sampler),
        });

    // Create the bind group layout for the font
    let bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: bind_group_layout_entries.as_slice(),
            label:   None,
        });

    // Create the bind group
    let bind_group = device.create_bind_group(
        &wgpu::BindGroupDescriptor {
            layout:  &bind_group_layout,
            entries: bind_group_entries.as_slice(),
            label:   None,
        });

    Fonts {
        fonts,
        bind_group,
        bind_group_layout,
    }
}

