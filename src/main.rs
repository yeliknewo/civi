#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate specs;
extern crate nalgebra;
extern crate genmesh;
extern crate rand;

use gfx::traits::FactoryExt;
use gfx::Device;

use genmesh::{Triangulate, Vertices};
use genmesh::generators::{SharedVertex, IndexedPolygon, Plane};

pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;

gfx_defines! {
    vertex Vertex {
        pos: [f32; 4] = "a_Pos",
        color: [f32; 3] = "a_Color",
    }

    constant Locals {
        transform: [[f32; 4]; 4] = "u_Transform",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        transform: gfx::Global<[[f32; 4]; 4]> = "u_Transform",
        locals: gfx::ConstantBuffer<Locals> = "Locals",
        out_color: gfx::RenderTarget<ColorFormat> = "Target0",
        out_depth: gfx::DepthTarget<DepthFormat> = gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

// const TRIANGLE: [Vertex; 3] = [
//     Vertex { pos: [ -0.5, -0.5, -5.0, 1.0 ], color: [1.0, 0.0, 0.0] },
//     Vertex { pos: [  0.5, -0.5, -5.0, 1.0 ], color: [0.0, 1.0, 0.0] },
//     Vertex { pos: [  0.0,  0.5, -5.0, 1.0 ], color: [0.0, 0.0, 1.0] }
// ];

const SQUARE_VERTEX_DATA: [Vertex; 4] = [
    Vertex { pos: [ -0.5, -0.5, 0.0, 1.0 ], color: [1.0, 0.0, 0.0] },
    Vertex { pos: [  0.5, -0.5, 0.0, 1.0 ], color: [0.0, 1.0, 0.0] },
    Vertex { pos: [ -0.5,  0.5, 0.0, 1.0 ], color: [0.0, 0.0, 1.0] },
    Vertex { pos: [  0.5,  0.5, 0.0, 1.0 ], color: [0.0, 0.0, 0.0] }
];

const SQUARE_INDEX_DATA: &'static [u16] = &[
    0, 1, 3,
    3, 2, 0,
];

const CLEAR_COLOR: [f32; 4] = [0.1, 0.2, 0.3, 1.0];

fn main() {
    let builder = glutin::WindowBuilder::new()
        .with_title("Civi")
        .with_dimensions(1024, 768)
        .with_vsync();

    let (window, mut device, mut factory, main_color, main_depth) = gfx_window_glutin::init::<ColorFormat, DepthFormat>(builder);

    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let pso = factory.create_pipeline_simple (
        include_bytes!("shader/civi_150_v.glsl"),
        include_bytes!("shader/civi_150_f.glsl"),
        pipe::new()
    ).unwrap();


    let width = 32usize;
    let height = 32usize;

    let half_width = width / 2;
    let half_height = height / 2;

    let plane = Plane::subdivide(width, height);

    let vertex_data: Vec<Vertex> = plane.shared_vertex_iter()
        .map(|(raw_x, raw_y)| {
            let vertex_x = half_width as f32 * raw_x;
            let vertex_y = half_height as f32 * raw_y;

            use rand::Rng;

            let mut rng = rand::thread_rng();

            let (a, b) = (rng.gen_range(0.0, 1.0), rng.gen_range(0.0, 1.0));

            Vertex {
                pos: [vertex_x + a, vertex_y + b, rng.gen_range(-a, b), 1.0],
                color: [
                    rng.gen_range(0.0, 1.0),
                    rng.gen_range(0.0, 1.0),
                    rng.gen_range(0.0, 1.0)
                ],
            }
        })
        .collect();

    let index_data: Vec<u32> = plane.indexed_polygon_iter()
        .triangulate()
        .vertices()
        .map(|i| i as u32)
        .collect();

    let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(&vertex_data, &index_data[..]);

    let model = {
        use nalgebra::ToHomogeneous;

        let translation = nalgebra::Vector3::new(0.0, 0.0, 0.0);

        let rotation = nalgebra::Vector3::new(0.0, 0.0, 0.0);

        nalgebra::Isometry3::new(translation, rotation).to_homogeneous()
    };

    let view = {
        use nalgebra::ToHomogeneous;

        let eye = nalgebra::Point3::new(0.0, 0.0, -95.0);

        let target = nalgebra::Point3::new(0.0, 0.0, 0.0);

        let up = nalgebra::Vector3::new(0.0, 1.0, 0.0);

        nalgebra::Isometry3::look_at_rh(&eye, &target, &up).to_homogeneous()
    };

    let perspective: nalgebra::Matrix4<f32> = *nalgebra::PerspectiveMatrix3::new(16.0 / 9.0, 75.0, 0.1, 100.0).as_matrix();

    let data = pipe::Data {
        vbuf: vertex_buffer,
        transform: *(perspective * view * model).as_ref(),
        locals: factory.create_constant_buffer(1),
        out_color: main_color,
        out_depth: main_depth,
    };

    'main: loop {
        for event in window.poll_events() {
            match event {
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Escape)) |
                glutin::Event::Closed => break 'main,
                _ => {},
            }
        }

        let locals = Locals { transform: data.transform };
        encoder.update_constant_buffer(&data.locals, &locals);
        encoder.clear(&data.out_color, CLEAR_COLOR);
        encoder.clear_depth(&data.out_depth, 1.0);
        encoder.draw(&slice, &pso, &data);
        encoder.flush(&mut device);
        window.swap_buffers().unwrap();
        device.cleanup();
    }
}
