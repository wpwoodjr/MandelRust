use futures::executor::block_on;
use bytemuck_derive::{Pod, Zeroable};
use wgpu::util::DeviceExt;

const WIDTH: u32 = 1600;
const HEIGHT: u32 = 1200;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Uniforms {
    xmin: f32,
    ymin: f32,
    xmax: f32,
    ymax: f32,
    max_iterations: u32,
}

async fn run(xmin: f32, ymin: f32, xmax: f32, ymax: f32, max_iterations: u32) {

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .unwrap();

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::downlevel_defaults(),
            },
            None,
        )
        .await
        .unwrap();

    let uniforms = Uniforms {
        xmin,
        ymin,
        xmax,
        ymax,
        max_iterations,
    };
    
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Uniform Buffer"),
        contents: bytemuck::bytes_of(&uniforms),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(include_str!("mandelbrot.wgsl").into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let bytes_per_pixel = 4; // Assuming 4 bytes per pixel (e.g., RGBA8 format)
    let aligned_bytes_per_row = ((WIDTH * bytes_per_pixel + 255) / 256) * 256;

    let output_buffer_size = (HEIGHT*aligned_bytes_per_row) as wgpu::BufferAddress;
    let storage_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Storage Buffer"),
        size: output_buffer_size,
        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: storage_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: uniform_buffer.as_entire_binding(),
            },
        ],
        label: None,
    });

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size: output_buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "main",
        label: None,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
        cpass.set_pipeline(&compute_pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.dispatch_workgroups(WIDTH / 8, HEIGHT / 8, 1);
    }

    encoder.copy_buffer_to_buffer(&storage_buffer, 0, &output_buffer, 0, output_buffer_size);
    queue.submit(Some(encoder.finish()));

    // Read the output texture into a buffer
    let buffer_slice = output_buffer.slice(..);
    use futures_intrusive::channel;

    // Sets the buffer up for mapping, sending over the result of the mapping back to us when it is finished.
    let (sender, receiver) = channel::shared::oneshot_channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
                
    device.poll(wgpu::Maintain::Wait);

    // Awaits until `buffer_future` can be read from
    if let Some(Ok(())) = receiver.receive().await {
        // Gets contents of buffer
        let padded_buffer = buffer_slice.get_mapped_range();

        // Save the image as a PNG
        let image = image::RgbaImage::from_raw(WIDTH, HEIGHT, padded_buffer.to_vec()).unwrap();
        image.save("mandelbrot.png").unwrap();

        // Clean up
        drop(padded_buffer);
        output_buffer.unmap();
    } else {
        eprintln!("Failed to map buffer");
    }
}

use clap::{App, Arg};
// ...

fn main() {
    env_logger::init();

    let matches = App::new("Mandelbrot WebGPU")
        .arg(
            Arg::with_name("xmin")
                .short('a')
                .long("xmin")
                .value_name("XMIN")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("ymin")
                .short('b')
                .long("ymin")
                .value_name("YMIN")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("xmax")
                .short('c')
                .long("xmax")
                .value_name("XMAX")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("ymax")
                .short('d')
                .long("ymax")
                .value_name("YMAX")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("max_iterations")
                .short('i')
                .long("max_iterations")
                .value_name("MAX_ITER")
                .takes_value(true)
                .required(true),
        )
        .get_matches();

    let xmin = matches
        .value_of_t::<f32>("xmin")
        .expect("XMIN must be a valid float");
    let ymin = matches
        .value_of_t::<f32>("ymin")
        .expect("YMIN must be a valid float");
    let xmax = matches
        .value_of_t::<f32>("xmax")
        .expect("XMAX must be a valid float");
    let ymax = matches
        .value_of_t::<f32>("ymax")
        .expect("YMAX must be a valid float");
    let max_iterations = matches
        .value_of_t::<u32>("max_iterations")
        .expect("MAX_ITER must be a valid integer");

    let future = run(xmin, ymin, xmax, ymax, max_iterations);
    block_on(future);
}
