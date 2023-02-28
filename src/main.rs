use std::num::NonZeroU64;
use std::collections::VecDeque;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

mod fonts;

const WIDTH:  u32 = 3840;
const HEIGHT: u32 = 2160;

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

struct Rng(u64);

impl Rng {
    fn rand(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 43;
        self.0
    }
}

#[pollster::main]
async fn main() {
    let mut rng = Rng(0x209e5223ce30b4);

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
        adapter.request_device(&Default::default(), None).await
            .expect("Failed to create Device")
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
    const COLS:    u32 = (WIDTH  + 3) / 4;
    const ROWS:    u32 = (HEIGHT + 5) / 6;
    const LINE_SZ: u32 = (COLS * 4 + 255) / 256 * 256;
    let text_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label:              None,
        size:               (LINE_SZ * ROWS) as u64,
        usage:              wgpu::BufferUsages::STORAGE |
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
                        has_dynamic_offset: true,
                        min_binding_size: NonZeroU64::new(LINE_SZ as u64),
                    }
                }
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
                            size:   NonZeroU64::new(LINE_SZ as u64),
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
                push_constant_ranges: &[],
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
            present_mode: wgpu::PresentMode::Immediate,
            alpha_mode:   wgpu::CompositeAlphaMode::Opaque,
            view_formats: Vec::new(),
        })
    });

    let mut buf = Vec::new();

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

        buf.clear();
        for row in 0..ROWS {
            for chr in 0..COLS / 2 {
                buf.extend_from_slice(&rng.rand().to_le_bytes());
            }
            while buf.len() % (LINE_SZ as usize) != 0 { buf.push(0); }
        }

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
            render_pass.set_bind_group(0, &fonts.bind_group, &[]);

            for line in 0..ROWS {
                render_pass.set_bind_group(1, &bind_group, &[line * LINE_SZ]);
                render_pass.draw(0..COLS * 6, line..line + 1);
            }
        }

        // Send the queue to the GPU
        queue.write_buffer(&text_buffer, 0, buf.as_slice());
        queue.submit(Some(commands.finish()));

        // Present the texture to the surface
        texture.present();

        while frame_times.len() >= 1024 {
            frame_times.pop_front();
        }

        frame_times.push_back(it.elapsed().as_secs_f64());

        if frame % 1024 == 0 {
            let fps = (frame_times.len() - 1) as f64 /
                (frame_times.back().unwrap() - frame_times.front().unwrap());
            println!("FPS {fps:10.4}");
        }
    }
}

