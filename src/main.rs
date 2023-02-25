use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

macro_rules! measure {
    ( $reason:expr, $tt:block ) => {
        {
            let it = std::time::Instant::now();
            let ret = $tt;
            let elapsed = it.elapsed().as_secs_f64() * 1000.;
            println!("[{elapsed:12.3} ms] {}", $reason);
            ret
        }
    }
}

#[pollster::main]
async fn main() {
    // Create an event loop
    let event_loop = EventLoop::new();

    // Create a window
    let window = WindowBuilder::new().build(&event_loop)
        .expect("Failed to build window");

    // Get an instance of wgpu
    let instance = measure!("Creating wgpu::Instance", {
        wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        })
    });

    // Create a surface for our window
    let surface = unsafe { instance.create_surface(&window) }
        .expect("Failed to create surface for window");

    // Get a physical card
    let adapter = measure!("Creating wgpu::Adapter", {
        instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            ..Default::default()
        }).await.unwrap()
    });

    // Get access to the device and a queue to issue commands to it
    let (device, queue) = measure!("Creating wgpu::Device", {
        adapter.request_device(&Default::default(), None).await.unwrap()
    });

    // Create the primary texture to render to
    let texture = measure!("Creating primary texture", {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Primary render texture"),
            size: wgpu::Extent3d {
                width: 1920,
                height: 1080,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    });

    let tv = texture.create_view(&Default::default());

    // Create a buffer that we can use to copy the primary texture to
    let buffer = measure!("Creating primary buffer", {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Primary render buffer"),
            size: 1920 * 1080 * 4,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        })
    });

    let bs = buffer.slice(..);

    measure!("Copy!", {
        for _ in 0..1000 {
            let mut commands = device.create_command_encoder(&Default::default());

            commands.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &tv,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.1, g: 0.5, b: 1.0, a: 1.0 }),
                            store: true,
                        },
                    })
                ],
                depth_stencil_attachment: None,
            });

            commands.copy_texture_to_buffer(
                wgpu::ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::ImageCopyBuffer {
                    buffer: &buffer,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: std::num::NonZeroU32::new(1920 * 4),
                        rows_per_image: None,
                    },
                },
                wgpu::Extent3d {
                    width: 1920,
                    height: 1080,
                    depth_or_array_layers: 1,
                },
            );

            let idx = queue.submit(Some(commands.finish()));

            bs.map_async(wgpu::MapMode::Read, |res| {
                res.unwrap();
            });
            device.poll(wgpu::MaintainBase::WaitForSubmissionIndex(idx));

            {
                let data = bs.get_mapped_range();
                std::fs::write("image_data.data", &data).unwrap();
            }

            buffer.unmap();
        }
    });
}

