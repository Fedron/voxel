use std::collections::HashMap;

use num_traits::FromPrimitive;

use crate::{
    mesh::Mesh,
    quad::{QuadFace, QuadFaceOptions},
    transform::Transform,
    utils::{coord_to_index, index_to_coord},
};

pub struct VoxelUniforms {
    pub view_projection: [[f32; 4]; 4],
    pub light_color: [f32; 3],
    pub light_position: [f32; 3],
}

#[derive(Copy, Clone)]
pub struct VoxelVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: VoxelColor,
}
implement_vertex!(VoxelVertex, position, normal, color);

pub type VoxelColor = [f32; 4];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Voxel {
    Air,
    Stone,
    Grass,
    Dirt,
    Water,
    Sand,
}

impl Into<VoxelColor> for Voxel {
    fn into(self) -> VoxelColor {
        match self {
            Voxel::Air => [0.0, 0.0, 0.0, 0.0],
            Voxel::Stone => [0.69, 0.72, 0.72, 1.0],
            Voxel::Grass => [0.23, 0.82, 0.24, 1.0],
            Voxel::Dirt => [0.63, 0.45, 0.29, 1.0],
            Voxel::Water => [0.0, 0.62, 1.0, 0.8],
            Voxel::Sand => [0.93, 0.89, 0.55, 1.0],
        }
    }
}

impl Voxel {
    pub fn is_air(self) -> bool {
        self == Voxel::Air
    }

    pub fn is_liquid(self) -> bool {
        self == Voxel::Water
    }

    pub fn is_solid(self) -> bool {
        !self.is_air() && !self.is_liquid()
    }
}

pub const CHUNK_SIZE: glam::UVec3 = glam::uvec3(16, 16, 16);

pub struct Chunk {
    grid_position: glam::UVec3,
    size: glam::UVec3,
    transform: Transform,
    voxels: Vec<Voxel>,
}

impl Chunk {
    pub fn new(grid_position: glam::UVec3, size: glam::UVec3) -> Self {
        let transform_position = grid_position * size;

        Self {
            grid_position,
            size,
            transform: Transform {
                position: transform_position.as_vec3(),
                rotation: glam::Quat::IDENTITY,
                scale: glam::Vec3::ONE,
            },
            voxels: vec![
                Voxel::Air;
                CHUNK_SIZE.x as usize * CHUNK_SIZE.y as usize * CHUNK_SIZE.z as usize
            ],
        }
    }

    pub fn get_voxel(&self, position: glam::UVec3) -> Option<&Voxel> {
        if position.x >= CHUNK_SIZE.x || position.y >= CHUNK_SIZE.y || position.z >= CHUNK_SIZE.z {
            return None;
        }

        let index = coord_to_index(position, CHUNK_SIZE);
        self.voxels.get(index)
    }

    pub fn set_voxel(&mut self, position: glam::UVec3, voxel: Voxel) {
        let index = coord_to_index(position, CHUNK_SIZE);
        if self.voxels.get(index).is_some() {
            self.voxels[index] = voxel;
        }
    }

    pub fn iter(&self) -> ChunkIterator {
        ChunkIterator {
            chunk: self,
            current_index: 0,
        }
    }

    pub fn transform(&self) -> Transform {
        self.transform
    }
}

pub struct ChunkIterator<'a> {
    chunk: &'a Chunk,
    current_index: usize,
}

impl<'a> Iterator for ChunkIterator<'a> {
    type Item = (&'a Voxel, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index < self.chunk.voxels.len() {
            let voxel = &self.chunk.voxels[self.current_index];
            let index = self.current_index;
            self.current_index += 1;
            Some((voxel, index))
        } else {
            None
        }
    }
}

pub struct ChunkMesh {
    pub solid: Mesh<VoxelVertex>,
    pub transparent: Option<Mesh<VoxelVertex>>,
}

pub struct ChunkMesher {}

impl ChunkMesher {
    pub fn mesh(chunk: &Chunk, neighbours: HashMap<glam::UVec3, &Chunk>) -> ChunkMesh {
        let mut solid_vertices = vec![];
        let mut solid_indices = vec![];
        let mut transparent_vertices = vec![];
        let mut transparent_indices = vec![];

        for (voxel, index) in chunk.iter() {
            match voxel {
                Voxel::Stone | Voxel::Grass | Voxel::Dirt | Voxel::Sand => {
                    let neighbours = Self::get_neighbouring_voxels(
                        chunk,
                        &neighbours,
                        index_to_coord(index, CHUNK_SIZE).as_ivec3(),
                        |v| !v.is_solid(),
                    );

                    for i in 0..6 {
                        if (neighbours >> i) & 1 == 1 {
                            let position = index_to_coord(index, CHUNK_SIZE);
                            let base_position =
                                glam::vec3(position.x as f32, position.y as f32, position.z as f32);
                            let mesh = QuadFace::from_i64(i as i64)
                                .expect("to convert primitive to quad face enum")
                                .as_mesh(QuadFaceOptions {
                                    base_position: base_position.into(),
                                    half_size: 0.5,
                                    color: (*voxel).into(),
                                    base_index: solid_vertices.len() as u32,
                                });
                            solid_vertices.extend(mesh.vertices);
                            solid_indices.extend(mesh.indices);
                        }
                    }
                }
                Voxel::Water => {
                    let neighbours = Self::get_neighbouring_voxels(
                        chunk,
                        &neighbours,
                        index_to_coord(index, CHUNK_SIZE).as_ivec3(),
                        Voxel::is_air,
                    );

                    for i in 0..6 {
                        if (neighbours >> i) & 1 == 1 {
                            let position = index_to_coord(index, CHUNK_SIZE);
                            let base_position =
                                glam::vec3(position.x as f32, position.y as f32, position.z as f32);
                            let mesh = QuadFace::from_i64(i as i64)
                                .expect("to convert primitive to quad face enum")
                                .as_mesh(QuadFaceOptions {
                                    base_position: base_position.into(),
                                    half_size: 0.5,
                                    color: (*voxel).into(),
                                    base_index: transparent_vertices.len() as u32,
                                });
                            transparent_vertices.extend(mesh.vertices);
                            transparent_indices.extend(mesh.indices);
                        }
                    }
                }
                _ => {}
            }
        }

        let transparent_mesh = if transparent_vertices.is_empty() {
            None
        } else {
            Some(Mesh {
                vertices: transparent_vertices,
                indices: transparent_indices,
            })
        };

        ChunkMesh {
            solid: Mesh {
                vertices: solid_vertices,
                indices: solid_indices,
            },
            transparent: transparent_mesh,
        }
    }

    fn get_neighbouring_voxels<C>(
        chunk: &Chunk,
        chunk_neighbours: &HashMap<glam::UVec3, &Chunk>,
        voxel_position: glam::IVec3,
        condition: C,
    ) -> u8
    where
        C: Fn(Voxel) -> bool,
    {
        let mut mask = 0;
        for i in 0..6 {
            let face_direction: glam::IVec3 = QuadFace::from_i64(i as i64)
                .expect("to convert primitive to quad face enum")
                .into();

            // If neighbour is within the same chunk, check voxel in the chunk
            let neighbour_position = voxel_position + face_direction;
            if neighbour_position.x >= 0
                && neighbour_position.x < chunk.size.x as i32
                && neighbour_position.y >= 0
                && neighbour_position.y < chunk.size.y as i32
                && neighbour_position.z >= 0
                && neighbour_position.z < chunk.size.z as i32
            {
                if let Some(neighbour) = chunk.get_voxel(neighbour_position.as_uvec3()) {
                    if condition(*neighbour) {
                        mask |= 1 << i;
                    }
                }
            }
            // If neighbour is out of bounds for this chunk, try checking the corresponding neighbouring chunk
            else {
                let neighbour_chunk_position = chunk.grid_position.as_ivec3() + face_direction;
                let neighbour_chunk_position: Result<glam::UVec3, _> =
                    neighbour_chunk_position.try_into();

                match neighbour_chunk_position {
                    Ok(neighbour) => {
                        if let Some(neighbour_chunk) = chunk_neighbours.get(&neighbour) {
                            let neighbour_position = match face_direction {
                                glam::IVec3::X => {
                                    glam::uvec3(0, voxel_position.y as u32, voxel_position.z as u32)
                                }
                                glam::IVec3::Y => {
                                    glam::uvec3(voxel_position.x as u32, 0, voxel_position.z as u32)
                                }
                                glam::IVec3::Z => {
                                    glam::uvec3(voxel_position.x as u32, voxel_position.y as u32, 0)
                                }
                                glam::IVec3::NEG_X => glam::uvec3(
                                    CHUNK_SIZE.x - 1,
                                    voxel_position.y as u32,
                                    voxel_position.z as u32,
                                ),
                                glam::IVec3::NEG_Y => glam::uvec3(
                                    voxel_position.x as u32,
                                    CHUNK_SIZE.y - 1,
                                    voxel_position.z as u32,
                                ),
                                glam::IVec3::NEG_Z => glam::uvec3(
                                    voxel_position.x as u32,
                                    voxel_position.y as u32,
                                    CHUNK_SIZE.z - 1,
                                ),
                                _ => unreachable!(),
                            };

                            if let Some(neighbour) = neighbour_chunk.get_voxel(neighbour_position) {
                                if condition(*neighbour) {
                                    mask |= 1 << i;
                                }
                            }
                        } else {
                            mask |= 1 << i;
                        }
                    }
                    Err(_) => {
                        mask |= 1 << i;
                    }
                }
            }
        }
        mask
    }
}
