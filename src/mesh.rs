#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}
implement_vertex!(Vertex, position, color);

pub struct Mesh<const V: usize, const I: usize> {
    pub vertices: [Vertex; V],
    pub indices: [u32; I],
}
