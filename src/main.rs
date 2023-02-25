use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

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

#[pollster::main]
async fn main() {
    // Create an event loop
    let event_loop = measure!("Creating winit::EventLoop", {
        EventLoop::new()
    });

    // Create a window
    let window = measure!("Creating winit::Window", {
        WindowBuilder::new()
            .with_inner_size(winit::dpi::PhysicalSize::new(800, 600))
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
            power_preference: wgpu::PowerPreference::HighPerformance,
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

    // Create render pipeline
    let render_pipeline = measure!("Creating render pipeline", {
        let render_pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
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
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
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

    loop {
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

            // Draw that shit!
            let val = rand::random::<u32>();
            render_pass.draw(0..4, val..val.checked_add(1).unwrap());
        }

        // Send the queue to the GPU
        queue.submit(Some(commands.finish()));

        // Present the texture to the surface
        texture.present();
    }
}

