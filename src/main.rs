#[macro_use]
extern crate glium;
use glium::Surface;
use mesh::Mesh;
use quad::QuadFace;

mod mesh;
mod quad;

fn main() {
    let event_loop = glium::winit::event_loop::EventLoop::builder()
        .build()
        .expect("event loop to be built");
    let (window, display) = glium::backend::glutin::SimpleWindowBuilder::new()
        .with_title("Voxels")
        .build(&event_loop);

    let quad: Mesh<4, 6> = QuadFace::Front.as_mesh(Default::default());
    let vertex_buffer =
        glium::VertexBuffer::new(&display, &quad.vertices).expect("to create vertex buffer");
    let indices = glium::index::IndexBuffer::new(
        &display,
        glium::index::PrimitiveType::TrianglesList,
        &quad.indices,
    )
    .expect("to create index buffer");

    let vertex_shader_src = r#"
        #version 140

        in vec3 position;
        in vec3 color;

        out vec3 vertex_color;

        uniform float offset;

        void main() {
            vec3 pos = position;
            pos.x += offset;

            vertex_color = color;
            gl_Position = vec4(pos, 1.0);
        }
    "#;

    let fragment_shader_src = r#"
        #version 140

        in vec3 vertex_color;
        out vec4 color;

        void main() {
            color = vec4(vertex_color, 1.0);
        }
    "#;

    let program =
        glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None)
            .expect("to compile shaders");

    let mut time: f32 = 0.0;

    #[allow(deprecated)]
    event_loop
        .run(move |event, window_target| {
            match event {
                glium::winit::event::Event::WindowEvent { event, .. } => match event {
                    glium::winit::event::WindowEvent::CloseRequested => window_target.exit(),
                    glium::winit::event::WindowEvent::RedrawRequested => {
                        time += 0.02;
                        let offset = time.sin() * 0.5;
                        let uniforms = uniform! { offset: offset };

                        let mut frame = display.draw();
                        frame.clear_color(1.0, 0.0, 1.0, 1.0);
                        frame
                            .draw(
                                &vertex_buffer,
                                &indices,
                                &program,
                                &uniforms,
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
