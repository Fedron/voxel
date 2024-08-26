use glium::glutin::surface::WindowSurface;

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

pub struct Mesh<V> {
    pub vertices: Vec<V>,
    pub indices: Vec<u32>,
}

impl<V> Mesh<V>
where
    V: glium::vertex::Vertex,
{
    pub fn as_opengl_buffers(
        &self,
        display: &glium::Display<WindowSurface>,
    ) -> Result<(glium::VertexBuffer<V>, glium::IndexBuffer<u32>), BufferCreationError> {
        let vertex_buffer = glium::VertexBuffer::new(display, &self.vertices)?;
        let index_buffer = glium::IndexBuffer::new(
            display,
            glium::index::PrimitiveType::TrianglesList,
            &self.indices,
        )?;

        Ok((vertex_buffer, index_buffer))
    }
}
