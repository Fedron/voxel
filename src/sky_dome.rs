use glium::{DrawParameters, Surface};

pub struct SkyDome {
    pub position: glam::Vec3,
    pub low_color: [f32; 3],
    pub high_color: [f32; 3],

    program: glium::Program,
    vertex_buffer: glium::VertexBuffer<SkyDomeVertex>,
    max_height: f32,
}

impl SkyDome {
    pub fn new(
        display: &glium::Display<glium::glutin::surface::WindowSurface>,
        rows: usize,
        cols: usize,
        radius: f32,
    ) -> Self {
        let vertex_buffer =
            Self::create_dome(display, rows, cols, radius).expect("to create dome vertex buffer");

        let program = glium::Program::from_source(
            display,
            include_str!("shaders/sky.vert"),
            include_str!("shaders/sky.frag"),
            None,
        )
        .expect("to compile sky dome shaders");

        Self {
            position: glam::Vec3::ZERO,
            low_color: [0.71, 0.85, 0.90],
            high_color: [0.0, 0.45, 0.74],

            program,
            vertex_buffer,
            max_height: radius,
        }
    }

    pub fn draw(&self, frame: &mut glium::Frame, view_projection: glam::Mat4) {
        let sky_dome_model = glam::Mat4::from_translation(self.position);
        frame
            .draw(
                &self.vertex_buffer,
                &glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
                &self.program,
                &glium::uniform! {
                    mvp: (view_projection * sky_dome_model).to_cols_array_2d(),
                    low_color: self.low_color,
                    high_color: self.high_color,
                    max_height: self.max_height,
                },
                &DrawParameters {
                    depth: glium::Depth {
                        test: glium::draw_parameters::DepthTest::IfLessOrEqual,
                        write: true,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .expect("to draw sky dome");
    }

    fn create_dome(
        display: &glium::Display<glium::glutin::surface::WindowSurface>,
        rows: usize,
        cols: usize,
        radius: f32,
    ) -> Result<glium::VertexBuffer<SkyDomeVertex>, glium::vertex::BufferCreationError> {
        let mut vertices = Vec::with_capacity((3 * cols) + (rows - 1) * (6 * cols));

        let pitch_angle = 90.0 / rows as f32;
        let heading_angle = 360.0 / cols as f32;

        let apex = glam::vec3(0.0, radius, 0.0);

        let pitch = -90.0;

        let mut heading = 0.0;
        while heading < 360.0 {
            vertices.push(SkyDomeVertex {
                position: apex.into(),
            });

            vertices.push(SkyDomeVertex {
                position: spherical_to_cartesian_coords(
                    radius,
                    pitch + pitch_angle,
                    heading + heading_angle,
                )
                .into(),
            });

            vertices.push(SkyDomeVertex {
                position: spherical_to_cartesian_coords(radius, pitch + pitch_angle, heading)
                    .into(),
            });

            heading += heading_angle;
        }

        let mut pitch = -90.0;
        while pitch < 0.0 {
            let mut heading = 0.0;
            while heading < 360.0 {
                let v0 = SkyDomeVertex {
                    position: spherical_to_cartesian_coords(radius, pitch, heading).into(),
                };

                let v1 = SkyDomeVertex {
                    position: spherical_to_cartesian_coords(radius, pitch, heading + heading_angle)
                        .into(),
                };

                let v2 = SkyDomeVertex {
                    position: spherical_to_cartesian_coords(radius, pitch + pitch_angle, heading)
                        .into(),
                };

                let v3 = SkyDomeVertex {
                    position: spherical_to_cartesian_coords(
                        radius,
                        pitch + pitch_angle,
                        heading + heading_angle,
                    )
                    .into(),
                };

                vertices.push(v0);
                vertices.push(v1);
                vertices.push(v2);

                vertices.push(v1);
                vertices.push(v3);
                vertices.push(v2);

                heading += heading_angle;
            }

            pitch += pitch_angle;
        }

        glium::VertexBuffer::new(display, &vertices)
    }
}

#[derive(Debug, Copy, Clone)]
struct SkyDomeVertex {
    pub position: [f32; 3],
}
implement_vertex!(SkyDomeVertex, position);

fn spherical_to_cartesian_coords(radius: f32, pitch: f32, heading: f32) -> glam::Vec3 {
    let pitch = pitch.to_radians();
    let heading = heading.to_radians();

    glam::vec3(
        radius * pitch.cos() * heading.sin(),
        -radius * pitch.sin(),
        radius * pitch.cos() * heading.cos(),
    )
}
