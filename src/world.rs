use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::mpsc::{Receiver, Sender},
    thread,
};

use glium::{DrawParameters, Surface};

use crate::{
    app::Window,
    chunk::{
        mesh::{Axis, Direction, Mesh, Vertex},
        Chunk, VoxelUniforms,
    },
    generation::WorldGenerationOptions,
    transform::{Matrix3x3, Matrix4x4},
};

struct Channel<T> {
    tx: Sender<T>,
    rx: Receiver<T>,
    in_process: HashSet<glam::IVec3>,
}

/// Represents the world.
pub struct World {
    render_distance: u8,
    /// Chunks in the world that have been generated.
    chunks: HashMap<glam::IVec3, Chunk>,

    chunk_generator_channel: Channel<Chunk>,
    chunk_meshing_channel: Channel<(glam::IVec3, Option<Mesh>, Option<Mesh>)>,

    /// Meshes for solid voxels of a chunk.
    chunk_solid_meshes:
        HashMap<glam::IVec3, (glium::VertexBuffer<Vertex>, glium::IndexBuffer<u32>)>,
    /// Meshes for transparent voxels of a chunk.
    chunk_transparent_meshes:
        HashMap<glam::IVec3, (glium::VertexBuffer<Vertex>, glium::IndexBuffer<u32>)>,
    /// Uniforms for a chunk.
    chunk_uniforms: HashMap<glam::IVec3, (Matrix4x4, Matrix3x3)>,

    window: Rc<Window>,
}

impl World {
    /// Creates a new empty world.
    pub fn new(window: Rc<Window>, render_distance: u8) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let chunk_generator_channel = Channel::<Chunk> {
            tx,
            rx,
            in_process: HashSet::new(),
        };

        let (tx, rx) = std::sync::mpsc::channel();
        let chunk_meshing_channel = Channel::<(glam::IVec3, Option<Mesh>, Option<Mesh>)> {
            tx,
            rx,
            in_process: HashSet::new(),
        };

        Self {
            render_distance,
            chunks: HashMap::new(),

            chunk_generator_channel,
            chunk_meshing_channel,

            chunk_solid_meshes: HashMap::new(),
            chunk_transparent_meshes: HashMap::new(),
            chunk_uniforms: HashMap::new(),

            window,
        }
    }

    /// Clears the world.
    pub fn clear(&mut self) {
        self.chunks.clear();
        self.chunk_solid_meshes.clear();
        self.chunk_transparent_meshes.clear();
        self.chunk_uniforms.clear();
    }

    /// Updates the world.
    ///
    /// This generates new chunks around the camera position.
    pub fn update(
        &mut self,
        camera_position: glam::Vec3,
        generation_options: &WorldGenerationOptions,
    ) {
        let center_chunk_pos = (camera_position / generation_options.chunk_size.as_vec3())
            .floor()
            .as_ivec3();

        for x in (center_chunk_pos.x - self.render_distance as i32)
            ..=(center_chunk_pos.x + self.render_distance as i32)
        {
            for y in (center_chunk_pos.y - self.render_distance as i32)
                ..=(center_chunk_pos.y + self.render_distance as i32)
            {
                for z in (center_chunk_pos.z - self.render_distance as i32)
                    ..=(center_chunk_pos.z + self.render_distance as i32)
                {
                    let chunk_pos = glam::IVec3::new(x, y, z);
                    if self.chunks.get(&chunk_pos).is_none()
                        && !self.chunk_generator_channel.in_process.contains(&chunk_pos)
                    {
                        self.chunk_generator_channel.in_process.insert(chunk_pos);

                        let tx = self.chunk_generator_channel.tx.clone();
                        let generation_options = generation_options.clone();
                        thread::spawn(move || {
                            let chunk =
                                crate::generation::generate_chunk(generation_options, chunk_pos);
                            tx.send(chunk)
                                .expect("to send generated chunk back to main thread");
                        });
                    }
                }
            }
        }

        if let Ok(chunk) = self.chunk_generator_channel.rx.try_recv() {
            self.chunk_generator_channel
                .in_process
                .remove(&chunk.grid_position);

            if !chunk.is_empty() {
                self.mesh_chunk(&chunk);
            }

            // Re-mesh neighbouring chunks
            let neighbours = self.get_neigbour_chunks(chunk.grid_position);
            for neighbour in neighbours.values() {
                self.mesh_chunk(neighbour);
            }

            self.chunks.insert(chunk.grid_position, chunk);
        }

        if let Ok((grid_position, solid_mesh, transparent_mesh)) =
            self.chunk_meshing_channel.rx.try_recv()
        {
            self.prepare_chunk_for_rendering(grid_position, solid_mesh, transparent_mesh);
        }
    }

    fn get_neigbour_chunks(&self, chunk_position: glam::IVec3) -> HashMap<glam::IVec3, Chunk> {
        let mut neighbours = HashMap::new();

        for axis in [Axis::X, Axis::Y, Axis::Z] {
            for direction in [Direction::Positive, Direction::Negative] {
                let neighbour_position = chunk_position + axis.get_normal(direction).as_ivec3();

                if let Some(neighbour) = self.chunks.get(&neighbour_position) {
                    neighbours.insert(neighbour_position, (*neighbour).clone());
                }
            }
        }

        neighbours
    }

    fn mesh_chunk(&mut self, chunk: &Chunk) {
        let neighbours = self.get_neigbour_chunks(chunk.grid_position);

        let tx = self.chunk_meshing_channel.tx.clone();
        let chunk = chunk.clone();
        thread::spawn(move || {
            let (solid_mesh, transparent_mesh) = chunk.mesh(&neighbours);
            tx.send((chunk.grid_position, solid_mesh, transparent_mesh))
                .expect("to send generated mesh back to main thread");
        });
    }

    fn prepare_chunk_for_rendering(
        &mut self,
        grid_position: glam::IVec3,
        solid_mesh: Option<Mesh>,
        transparent_mesh: Option<Mesh>,
    ) {
        if let Some(solid_mesh) = solid_mesh {
            let vertex_buffer = solid_mesh
                .vertex_buffer(&self.window.display)
                .expect("to create vertex buffer");
            let index_buffer = solid_mesh
                .index_buffer(&self.window.display)
                .expect("to create index buffer");
            self.chunk_solid_meshes
                .insert(grid_position, (vertex_buffer, index_buffer));
        }

        if let Some(transparent_mesh) = transparent_mesh {
            let vertex_buffer = transparent_mesh
                .vertex_buffer(&self.window.display)
                .expect("to create vertex buffer");
            let index_buffer = transparent_mesh
                .index_buffer(&self.window.display)
                .expect("to create index buffer");
            self.chunk_transparent_meshes
                .insert(grid_position, (vertex_buffer, index_buffer));
        }

        let chunk = self.chunks.get(&grid_position).unwrap();
        self.chunk_uniforms.insert(
            grid_position,
            (
                chunk.transform().model_matrix().to_cols_array_2d(),
                chunk.transform().normal_matrix().to_cols_array_2d(),
            ),
        );
    }

    /// Draws the world.
    pub fn draw(
        &self,
        frame: &mut glium::Frame,
        shader: &glium::Program,
        uniforms: VoxelUniforms,
        draw_wireframe: bool,
    ) {
        for (position, (vertex_buffer, index_buffer)) in self.chunk_solid_meshes.iter() {
            let (model, normal) = self.chunk_uniforms.get(position).unwrap();

            frame
                .draw(
                    vertex_buffer,
                    index_buffer,
                    &shader,
                    &uniform! {
                        view_proj: uniforms.view_projection,
                        model: *model,
                        normal_matrix: *normal,
                        light_color: uniforms.light_color,
                        light_position: uniforms.light_position
                    },
                    &DrawParameters {
                        polygon_mode: if draw_wireframe {
                            glium::draw_parameters::PolygonMode::Line
                        } else {
                            glium::draw_parameters::PolygonMode::Fill
                        },
                        depth: glium::Depth {
                            test: glium::draw_parameters::DepthTest::IfLess,
                            write: true,
                            ..Default::default()
                        },
                        backface_culling:
                            glium::draw_parameters::BackfaceCullingMode::CullCounterClockwise,
                        blend: glium::Blend::alpha_blending(),
                        ..Default::default()
                    },
                )
                .expect("to draw vertices");
        }

        for (position, (vertex_buffer, index_buffer)) in self.chunk_transparent_meshes.iter() {
            let (model, normal) = self.chunk_uniforms.get(position).unwrap();

            frame
                .draw(
                    vertex_buffer,
                    index_buffer,
                    &shader,
                    &uniform! {
                        view_proj: uniforms.view_projection,
                        model: *model,
                        normal_matrix: *normal,
                        light_color: uniforms.light_color,
                        light_position: uniforms.light_position
                    },
                    &DrawParameters {
                        polygon_mode: if draw_wireframe {
                            glium::draw_parameters::PolygonMode::Line
                        } else {
                            glium::draw_parameters::PolygonMode::Fill
                        },
                        depth: glium::Depth {
                            test: glium::draw_parameters::DepthTest::IfLess,
                            write: true,
                            ..Default::default()
                        },
                        backface_culling:
                            glium::draw_parameters::BackfaceCullingMode::CullCounterClockwise,
                        blend: glium::Blend::alpha_blending(),
                        ..Default::default()
                    },
                )
                .expect("to draw vertices");
        }
    }
}
