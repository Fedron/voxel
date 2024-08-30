#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
}
implement_vertex!(Vertex, position, normal, color);

#[derive(Debug, Clone, Copy)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub fn get_normal(&self, direction: Direction) -> glam::Vec3 {
        match self {
            Axis::X => match direction {
                Direction::Positive => glam::Vec3::X,
                Direction::Negative => glam::Vec3::NEG_X,
            },
            Axis::Y => match direction {
                Direction::Positive => glam::Vec3::Y,
                Direction::Negative => glam::Vec3::NEG_Y,
            },
            Axis::Z => match direction {
                Direction::Positive => glam::Vec3::Z,
                Direction::Negative => glam::Vec3::NEG_Z,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    Positive,
    Negative,
}

pub struct Mesh {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl Mesh {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn vertex_buffer(
        &self,
        display: &glium::Display<glium::glutin::surface::WindowSurface>,
    ) -> Result<glium::VertexBuffer<Vertex>, glium::vertex::BufferCreationError> {
        glium::VertexBuffer::new(display, &self.vertices)
    }

    pub fn index_buffer(
        &self,
        display: &glium::Display<glium::glutin::surface::WindowSurface>,
    ) -> Result<glium::IndexBuffer<u32>, glium::index::BufferCreationError> {
        glium::IndexBuffer::new(
            display,
            glium::index::PrimitiveType::TrianglesList,
            &self.indices,
        )
    }

    pub fn add_quad<P, N, C>(&mut self, p1: P, p2: P, p3: P, p4: P, normal: N, color: C)
    where
        P: Into<[f32; 3]>,
        N: Into<[f32; 3]> + Copy,
        C: Into<[f32; 4]> + Copy,
    {
        let start_index = self.vertices.len() as u32;
        self.vertices.extend(&[
            Vertex {
                position: p1.into(),
                normal: normal.into(),
                color: color.into(),
            },
            Vertex {
                position: p2.into(),
                normal: normal.into(),
                color: color.into(),
            },
            Vertex {
                position: p3.into(),
                normal: normal.into(),
                color: color.into(),
            },
            Vertex {
                position: p4.into(),
                normal: normal.into(),
                color: color.into(),
            },
        ]);
        self.indices.extend(&[
            start_index,
            start_index + 1,
            start_index + 2,
            start_index,
            start_index + 2,
            start_index + 3,
        ]);
    }

    pub fn add_face<C>(
        &mut self,
        position: glam::Vec3,
        size: glam::Vec2,
        axis: Axis,
        direction: Direction,
        color: C,
    ) where
        C: Into<[f32; 4]> + Copy,
    {
        let vertices = match (axis, direction) {
            (Axis::X, Direction::Positive) => [
                [position.x, position.y, position.z + size.y],
                [position.x, position.y + size.x, position.z + size.y],
                [position.x, position.y + size.x, position.z],
                [position.x, position.y, position.z],
            ],

            (Axis::X, Direction::Negative) => [
                [position.x, position.y, position.z],
                [position.x, position.y + size.x, position.z],
                [position.x, position.y + size.x, position.z + size.y],
                [position.x, position.y, position.z + size.y],
            ],
            (Axis::Y, Direction::Positive) => [
                [position.x, position.y, position.z],
                [position.x + size.x, position.y, position.z],
                [position.x + size.x, position.y, position.z + size.y],
                [position.x, position.y, position.z + size.y],
            ],

            (Axis::Y, Direction::Negative) => [
                [position.x, position.y, position.z],
                [position.x, position.y, position.z + size.y],
                [position.x + size.x, position.y, position.z + size.y],
                [position.x + size.x, position.y, position.z],
            ],

            (Axis::Z, Direction::Positive) => [
                [position.x, position.y, position.z],
                [position.x, position.y + size.y, position.z],
                [position.x + size.x, position.y + size.y, position.z],
                [position.x + size.x, position.y, position.z],
            ],

            (Axis::Z, Direction::Negative) => [
                [position.x, position.y, position.z],
                [position.x + size.x, position.y, position.z],
                [position.x + size.x, position.y + size.y, position.z],
                [position.x, position.y + size.y, position.z],
            ],
        };

        self.add_quad(
            vertices[0],
            vertices[1],
            vertices[2],
            vertices[3],
            axis.get_normal(direction),
            color,
        );
    }
}
