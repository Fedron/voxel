use glium::glutin::surface::WindowSurface;

#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
}
implement_vertex!(Vertex, position, normal, color);

#[derive(Debug, Clone, Copy)]
pub enum BufferCreationError {
    VertexBufferCreationError,
    IndexBufferCreationError,
}

impl From<glium::vertex::BufferCreationError> for BufferCreationError {
    fn from(_error: glium::vertex::BufferCreationError) -> Self {
        BufferCreationError::VertexBufferCreationError
    }
}

impl From<glium::index::BufferCreationError> for BufferCreationError {
    fn from(_error: glium::index::BufferCreationError) -> Self {
        BufferCreationError::IndexBufferCreationError
    }
}

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn as_opengl_buffers(
        &self,
        display: &glium::Display<WindowSurface>,
    ) -> Result<(glium::VertexBuffer<Vertex>, glium::IndexBuffer<u32>), BufferCreationError> {
        let vertex_buffer = glium::VertexBuffer::new(display, &self.vertices)?;
        let index_buffer = glium::IndexBuffer::new(
            display,
            glium::index::PrimitiveType::TrianglesList,
            &self.indices,
        )?;

        Ok((vertex_buffer, index_buffer))
    }
}
