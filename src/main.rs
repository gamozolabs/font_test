mod fonts;

use std::mem::size_of;
use std::num::NonZeroU64;
use std::collections::VecDeque;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;
use fonts::FontSize;

const WIDTH:  u32 = 1440;
const HEIGHT: u32 =  900;

/// Log how long it took to execute `$tt`, using `$reason` as the status
/// message of what was timed
macro_rules! measure {
    ( $reason:expr, $tt:block ) => {
        {
            let it = std::time::Instant::now();
            let ret = $tt;
            let elapsed = it.elapsed().as_secs_f64() * 1000.;
            println!("[{elapsed:8.3} ms] {}", $reason);
            ret
        }
    }
}

unsafe trait Castable: Copy {
    fn cast(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self as *const Self as *const u8,
                std::mem::size_of_val(self))
        }
    }
}

unsafe impl Castable for (f32, f32) {}

#[derive(Clone, Copy)]
#[repr(C)]
struct Globals {
    win_width:  f32,
    win_height: f32,
}

unsafe impl Castable for Globals {}

#[derive(Clone, Copy)]
#[repr(C)]
struct PushConstants {
    rgba:      (f32, f32, f32, f32),
    xy:        (f32, f32),
    offset:    u32,
}

unsafe impl Castable for PushConstants {}

#[pollster::main]
async fn main() {
    // Create an event loop
    let event_loop = measure!("Creating winit::EventLoop", {
        EventLoop::new()
    });

    // Create a window
    let window = measure!("Creating winit::Window", {
        WindowBuilder::new()
            .with_inner_size(winit::dpi::PhysicalSize::new(WIDTH, HEIGHT))
            .with_resizable(false)
            .build(&event_loop)
            .expect("Failed to build window")
    });

    // Get an instance of wgpu
    let instance = measure!("Creating wgpu::Instance", {
        wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        })
    });

    // Create a surface for our window
    let surface = measure!("Creating wgpu::Surface", {
        unsafe { instance.create_surface(&window) }
            .expect("Failed to create surface for window")
    });

    // Get a physical card
    let adapter = measure!("Creating wgpu::Adapter", {
        instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference:   wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            ..Default::default()
        }).await.expect("Failed to create Adapter")
    });

    // Get access to the device and a queue to issue commands to it
    let (device, queue) = measure!("Creating wgpu::Device", {
        adapter.request_device(&wgpu::DeviceDescriptor {
            features: wgpu::Features::PUSH_CONSTANTS,
            limits:   wgpu::Limits {
                max_push_constant_size: size_of::<PushConstants>() as u32,
                ..Default::default()
            },
            ..Default::default()
        }, None).await.expect("Failed to create Device")
    });

    // Compile the shader
    let shader = measure!("Compiling shader", {
        device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"))
    });

    // Load the font
    let fonts = measure!("Loading fonts", {
        fonts::load_fonts(&device, &queue)
    });

    // Create buffer for text
    const FONT_BUFFER_SIZE: u64 = 1024 * 1024;
    let text_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label:              None,
        size:               FONT_BUFFER_SIZE,
        usage:              wgpu::BufferUsages::STORAGE |
                            wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Create buffer for globals to the shader
    let global_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label:              None,
        size:               size_of::<Globals>() as u64,
        usage:              wgpu::BufferUsages::UNIFORM |
                            wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Create the bind group layout for the font
    let bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: true,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(FONT_BUFFER_SIZE),
                    }
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size:
                            NonZeroU64::new(size_of::<Globals>() as u64),
                    }
                },
            ],
            label:   None,
        });

    // Create the bind group
    let bind_group = device.create_bind_group(
        &wgpu::BindGroupDescriptor {
            layout:  &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding:  0,
                    resource: wgpu::BindingResource::Buffer(
                        wgpu::BufferBinding {
                            buffer: &text_buffer,
                            offset: 0,
                            size:   NonZeroU64::new(FONT_BUFFER_SIZE),
                        }
                    ),
                },
                wgpu::BindGroupEntry {
                    binding:  1,
                    resource: wgpu::BindingResource::Buffer(
                        wgpu::BufferBinding {
                            buffer: &global_buffer,
                            offset: 0,
                            size:
                                NonZeroU64::new(size_of::<Globals>() as u64),
                        }
                    ),
                },
            ],
            label:   None,
        });

    // Create render pipeline
    let render_pipeline = measure!("Creating render pipeline", {
        let render_pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &fonts.bind_group_layout,
                    &bind_group_layout,
                ],
                push_constant_ranges: &[
                    wgpu::PushConstantRange {
                        stages: wgpu::ShaderStages::VERTEX,
                        range:  0..size_of::<PushConstants>() as u32,
                    }
                ],
            });
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        })
    });

    // Configure the surface
    measure!("Configuring wgpu::Surface", {
        surface.configure(&device, &wgpu::SurfaceConfiguration {
            usage:        wgpu::TextureUsages::RENDER_ATTACHMENT,
            format:       wgpu::TextureFormat::Bgra8UnormSrgb,
            width:        window.inner_size().width,
            height:       window.inner_size().height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode:   wgpu::CompositeAlphaMode::Opaque,
            view_formats: Vec::new(),
        })
    });

    let globals = Globals {
        win_width:  WIDTH  as f32,
        win_height: HEIGHT as f32,
    };
    queue.write_buffer(&global_buffer, 0, globals.cast());

    let all_fonts = [
        FontSize::Size4x6,
        FontSize::Size6x8,
        FontSize::Size6x10,
        FontSize::Size8x12,
        FontSize::Size8x14,
        FontSize::Size8x15,
        FontSize::Size8x16,
        FontSize::Size12x20,
        FontSize::Size16x24,
        FontSize::Size24x36,
    ];
    let mut strings = Vec::new();
    for _ in 0..100 {
        strings.push((
            all_fonts[rand::random::<usize>() % all_fonts.len()],
            PushConstants {
                xy:     ((rand::random::<u32>() % WIDTH) as f32, (rand::random::<u32>() % HEIGHT) as f32),
                rgba:   (rand::random::<f32>(), rand::random::<f32>(), rand::random::<f32>(), rand::random::<f32>()),
                offset: 0,
            },
            b"Hello world".as_slice(),
        ));
    }

    // Allocate all text data in one big buffer
    let mut text_data = Vec::new();
    for (_, pc, msg) in &mut strings {
        pc.offset = text_data.len() as u32 / 4;
        text_data.extend_from_slice(*msg);
        text_data.resize((text_data.len() + 3) & !3, 0u8);
    }
    queue.write_buffer(&text_buffer, 0, text_data.as_slice());

    let mut frame_times = VecDeque::new();
    let it = std::time::Instant::now();
    for frame in 0u64.. {
        // Get the current vsync texture to present to for the surface (window)
        let texture = surface.get_current_texture()
            .expect("Failed to get current texture");

        // Create a view of the texture
        let tv = texture.texture.create_view(&Default::default());

        // Create a new command encoder
        let mut commands = device.create_command_encoder(&Default::default());

        {
            // Start a render pass, clearing the screen to black
            let mut render_pass = commands.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[
                        Some(wgpu::RenderPassColorAttachment {
                            view: &tv,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.0, g: 0.0, b: 0.0, a: 1.0
                                }),
                                store: true,
                            },
                        })
                    ],
                    depth_stencil_attachment: None,
                });

            render_pass.set_pipeline(&render_pipeline);
            render_pass.set_bind_group(1, &bind_group, &[]);

            for (font_size, push_constants, msg) in &strings {
                // Set the font size
                render_pass.set_bind_group(0,
                    &fonts.fonts[*font_size as usize].bind_group, &[]);

                // Write the constants
                render_pass.set_push_constants(
                    wgpu::ShaderStages::VERTEX, 0, push_constants.cast());

                // Draw the text
                render_pass.draw(0..(msg.len() * 6) as u32, 0..1);
            }
        }

        // Send the queue to the GPU
        queue.submit(Some(commands.finish()));

        // Present the texture to the surface
        texture.present();

        while frame_times.len() >= 128 {
            frame_times.pop_front();
        }

        frame_times.push_back(it.elapsed().as_secs_f64());

        if frame % 128 == 0 {
            let fps = (frame_times.len() - 1) as f64 /
                (frame_times.back().unwrap() - frame_times.front().unwrap());
            println!("FPS {fps:10.4}");
        }
    }
}

