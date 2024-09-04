use std::{collections::HashMap, rc::Rc};

use glium::{DrawParameters, Surface};

use crate::{
    app::Window,
    chunk::{
        mesh::{Axis, Direction, MeshBuffers},
        Chunk, ChunkMesher, VoxelUniforms,
    },
    generator::WorldGenerator,
};

type ModelMatrix = [[f32; 4]; 4];
type NormalMatrix = [[f32; 3]; 3];

pub struct World {
    chunks: HashMap<glam::IVec3, Chunk>,

    chunk_solid_meshes: HashMap<glam::IVec3, MeshBuffers>,
    chunk_transparent_meshes: HashMap<glam::IVec3, MeshBuffers>,
    chunk_uniforms: HashMap<glam::IVec3, (ModelMatrix, NormalMatrix)>,

    window: Rc<Window>,
}

impl World {
    pub fn new(window: Rc<Window>) -> Self {
        Self {
            chunks: HashMap::new(),

            chunk_solid_meshes: HashMap::new(),
            chunk_transparent_meshes: HashMap::new(),
            chunk_uniforms: HashMap::new(),

            window,
        }
    }

    pub fn update(&mut self, camera_position: glam::Vec3, generator: &WorldGenerator) {
        let current_chunk_pos = (camera_position / generator.options.chunk_size.as_vec3())
            .floor()
            .as_ivec3();

        if self.chunks.get(&current_chunk_pos).is_none() {
            let chunk = generator.generate_chunk(current_chunk_pos);

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

                let (solid_mesh, transparent_mesh) = ChunkMesher::mesh(&chunk, &neighbours);

                if let Some(solid_mesh) = solid_mesh {
                    let buffers = solid_mesh.as_buffers(&self.window.display);
                    self.chunk_solid_meshes.insert(current_chunk_pos, buffers);
                }

                if let Some(transparent_mesh) = transparent_mesh {
                    let buffers = transparent_mesh.as_buffers(&self.window.display);
                    self.chunk_transparent_meshes
                        .insert(current_chunk_pos, buffers);
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

    pub fn draw(
        &self,
        frame: &mut glium::Frame,
        shader: &glium::Program,
        uniforms: VoxelUniforms,
        draw_wireframe: bool,
    ) {
        for (position, mesh) in self.chunk_solid_meshes.iter() {
            let (model, normal) = self.chunk_uniforms.get(position).unwrap();

            frame
                .draw(
                    &mesh.vertex_buffer,
                    &mesh.index_buffer,
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

        for (position, mesh) in self.chunk_transparent_meshes.iter() {
            let (model, normal) = self.chunk_uniforms.get(position).unwrap();

            frame
                .draw(
                    &mesh.vertex_buffer,
                    &mesh.index_buffer,
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
