use num_traits::FromPrimitive;

use crate::{
    mesh::Mesh,
    quad::{QuadFace, QuadFaceOptions},
    transform::Transform,
    utils::{coord_to_index, index_to_coord},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Voxel {
    Air,
    Stone,
}

pub const CHUNK_SIZE: glam::UVec3 = glam::uvec3(16, 16, 16);

pub struct Chunk {
    transform: Transform,
    voxels: Vec<Voxel>,
}

impl Chunk {
    pub fn new(position: glam::UVec3) -> Self {
        Self {
            transform: Transform {
                position: glam::vec3(position.x as f32, position.y as f32, position.z as f32),
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

pub struct ChunkMesher {}

impl ChunkMesher {
    pub fn mesh(chunk: &Chunk) -> Mesh {
        let mut vertices = vec![];
        let mut indices = vec![];

        for (voxel, index) in chunk.iter() {
            match voxel {
                Voxel::Stone => {
                    let neighbours = Self::get_neighbouring_voxels(
                        chunk,
                        index_to_coord(index, CHUNK_SIZE),
                        Voxel::Air,
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
                                    color: [0.5, 0.5, 0.5],
                                    base_index: vertices.len() as u32,
                                });
                            vertices.extend(mesh.vertices);
                            indices.extend(mesh.indices);
                        }
                    }
                }
                _ => {}
            }
        }

        Mesh { vertices, indices }
    }

    fn get_neighbouring_voxels(chunk: &Chunk, position: glam::UVec3, voxel: Voxel) -> u8 {
        let mut mask = 0;
        for i in 0..6 {
            let face_direction: glam::IVec3 = QuadFace::from_i64(i as i64)
                .expect("to convert primitive to quad face enum")
                .into();
            let neighbour = position.saturating_add_signed(face_direction);
            if let Some(v) = chunk.get_voxel(neighbour) {
                if *v == voxel {
                    mask |= 1 << i;
                }
            }
        }
        mask
    }
}
