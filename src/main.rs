#[macro_use]
extern crate glium;
use camera::Camera;
use glium::Surface;
use mesh::Mesh;
use quad::QuadFace;
use utils::degrees_to_radians;

mod camera;
mod mesh;
mod quad;
mod utils;

fn main() {
    let event_loop = glium::winit::event_loop::EventLoop::builder()
        .build()
        .expect("event loop to be built");
    let (window, display) = glium::backend::glutin::SimpleWindowBuilder::new()
        .with_title("Voxels")
        .build(&event_loop);
    let window_size = window.inner_size();

    let camera = Camera {
        eye: (0.0, 0.0, 10.0).into(),
        target: glam::Vec3::ZERO,
        up: glam::Vec3::Y,
        aspect: window_size.width as f32 / window_size.height as f32,
        fovy: degrees_to_radians(45.0),
        near_plane: 0.1,
        far_plane: 1000.0,
    };

    let quad: Mesh<4, 6> = QuadFace::Front.as_mesh(Default::default());
    let vertex_buffer =
        glium::VertexBuffer::new(&display, &quad.vertices).expect("to create vertex buffer");
    let indices = glium::index::IndexBuffer::new(
        &display,
        glium::index::PrimitiveType::TrianglesList,
        &quad.indices,
    )
    .expect("to create index buffer");

    let program = glium::Program::from_source(
        &display,
        include_str!("shaders/shader.vert"),
        include_str!("shaders/shader.frag"),
        None,
    )
    .expect("to compile shaders");

    #[allow(deprecated)]
    event_loop
        .run(move |event, window_target| {
            match event {
                glium::winit::event::Event::WindowEvent { event, .. } => match event {
                    glium::winit::event::WindowEvent::CloseRequested => window_target.exit(),
                    glium::winit::event::WindowEvent::RedrawRequested => {
                        let view_proj = camera.build_view_projection_matrix().to_cols_array_2d();

                        let mut frame = display.draw();
                        frame.clear_color(1.0, 0.0, 1.0, 1.0);
                        frame
                            .draw(
                                &vertex_buffer,
                                &indices,
                                &program,
                                &uniform! { view_proj: view_proj},
                                &Default::default(),
                            )
                            .expect("to draw vertices");
                        frame.finish().expect("to finish drawing");
                    }
                    glium::winit::event::WindowEvent::Resized(window_size) => {
                        display.resize(window_size.into());
                    }
                    _ => (),
                },
                glium::winit::event::Event::AboutToWait => {
                    window.request_redraw();
                }
                _ => (),
            };
        })
        .unwrap();
}
