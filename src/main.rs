#[macro_use]
extern crate glium;
use glium::Surface;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 3],
}
implement_vertex!(Vertex, position);

fn main() {
    let event_loop = glium::winit::event_loop::EventLoop::builder()
        .build()
        .expect("event loop to be built");
    let (_window, display) = glium::backend::glutin::SimpleWindowBuilder::new()
        .with_title("Voxels")
        .build(&event_loop);

    let triangle = vec![
        Vertex {
            position: [-0.5, -0.5, 0.0],
        },
        Vertex {
            position: [0.0, 0.5, 0.0],
        },
        Vertex {
            position: [0.5, -0.5, 0.0],
        },
    ];
    let vertex_buffer = glium::VertexBuffer::new(&display, &triangle).unwrap();
    let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

    let vertex_shader_src = r#"
        #version 140

        in vec3 position;

        void main() {
            gl_Position = vec4(position, 1.0);
        }
    "#;

    let fragment_shader_src = r#"
        #version 140

        out vec4 color;

        void main() {
            color = vec4(0.0, 1.0, 1.0, 1.0);
        }
    "#;

    let program =
        glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None)
            .expect("to compile shaders");

    let mut frame = display.draw();
    frame.clear_color(1.0, 0.0, 1.0, 1.0);
    frame
        .draw(
            &vertex_buffer,
            &indices,
            &program,
            &glium::uniforms::EmptyUniforms,
            &Default::default(),
        )
        .expect("to draw vertices");
    frame.finish().expect("to finish drawing");

    #[allow(deprecated)]
    event_loop
        .run(move |event, window_target| {
            match event {
                glium::winit::event::Event::WindowEvent { event, .. } => match event {
                    glium::winit::event::WindowEvent::CloseRequested => window_target.exit(),
                    _ => (),
                },
                _ => (),
            };
        })
        .unwrap();
}
