use std::{collections::HashMap, rc::Rc};

use glium::{DrawParameters, Surface};

use crate::{
    app::Window,
    chunk::{
        mesh::{Axis, Direction, Vertex},
        Chunk, VoxelUniforms,
    },
    generation::WorldGenerationOptions,
    transform::{Matrix3x3, Matrix4x4},
};

/// Represents the world.
pub struct World {
    /// Chunks in the world that have been generated.
    chunks: HashMap<glam::IVec3, Chunk>,

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
    pub fn new(window: Rc<Window>) -> Self {
        Self {
            chunks: HashMap::new(),

            chunk_solid_meshes: HashMap::new(),
            chunk_transparent_meshes: HashMap::new(),
            chunk_uniforms: HashMap::new(),

            window,
        }
    }

    /// Updates the world.
    ///
    /// This generates new chunks around the camera position.
    pub fn update(
        &mut self,
        camera_position: glam::Vec3,
        generation_options: &WorldGenerationOptions,
    ) {
        let current_chunk_pos = (camera_position / generation_options.chunk_size.as_vec3())
            .floor()
            .as_ivec3();

        if self.chunks.get(&current_chunk_pos).is_none() {
            let chunk = crate::generation::generate_chunk(generation_options, current_chunk_pos);

            if !chunk.is_empty() {
                let mut neighbours = HashMap::new();

                for axis in [Axis::X, Axis::Y, Axis::Z] {
                    for direction in [Direction::Positive, Direction::Negative] {
                        let neighbour_position =
                            current_chunk_pos + axis.get_normal(direction).as_ivec3();

                        if let Some(neighbour) = self.chunks.get(&neighbour_position) {
                            neighbours.insert(neighbour_position, neighbour);
                        }
                    }
                }

                let (solid_mesh, transparent_mesh) = chunk.mesh(&neighbours);

                if let Some(solid_mesh) = solid_mesh {
                    let vertex_buffer = solid_mesh
                        .vertex_buffer(&self.window.display)
                        .expect("to create vertex buffer");
                    let index_buffer = solid_mesh
                        .index_buffer(&self.window.display)
                        .expect("to create index buffer");
                    self.chunk_solid_meshes
                        .insert(current_chunk_pos, (vertex_buffer, index_buffer));
                }

                if let Some(transparent_mesh) = transparent_mesh {
                    let vertex_buffer = transparent_mesh
                        .vertex_buffer(&self.window.display)
                        .expect("to create vertex buffer");
                    let index_buffer = transparent_mesh
                        .index_buffer(&self.window.display)
                        .expect("to create index buffer");
                    self.chunk_transparent_meshes
                        .insert(current_chunk_pos, (vertex_buffer, index_buffer));
                }

                self.chunk_uniforms.insert(
                    current_chunk_pos,
                    (
                        chunk.transform().model_matrix().to_cols_array_2d(),
                        chunk.transform().normal_matrix().to_cols_array_2d(),
                    ),
                );

                self.chunks.insert(current_chunk_pos, chunk);
            }
        }
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
