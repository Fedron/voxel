use glium::Surface;

fn main() {
    let event_loop = glium::winit::event_loop::EventLoop::builder()
        .build()
        .expect("event loop to be built");
    let (_window, display) = glium::backend::glutin::SimpleWindowBuilder::new()
        .with_title("Voxels")
        .build(&event_loop);

    let mut frame = display.draw();
    frame.clear_color(1.0, 0.0, 1.0, 1.0);
    frame.finish().unwrap();

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
