use bon::bon;
use glam::FloatExt;
use noise::{core::perlin::perlin_2d, permutationtable::PermutationTable};

use crate::chunk::{Chunk, Voxel};

pub struct WorldGenerator {
    chunk_size: glam::UVec3,
    max_world_height: u32,

    permutation_table: PermutationTable,
}

#[bon]
impl WorldGenerator {
    #[builder]
    pub fn new(seed: u32, chunk_size: glam::UVec3, max_world_height: u32) -> Self {
        let permutation_table = PermutationTable::new(seed);

        Self {
            chunk_size,
            max_world_height,
            permutation_table,
        }
    }

    pub fn generate_chunk(&self, position: glam::UVec3) -> Chunk {
        let mut chunk = Chunk::new(position);

        for x in 0..self.chunk_size.x {
            for z in 0..self.chunk_size.z {
                let height = perlin_2d(
                    (
                        (position.x + x) as f64 / 64.0,
                        (position.z + z) as f64 / 64.0,
                    )
                        .into(),
                    &self.permutation_table,
                )
                .remap(-1.0, 1.0, 0.0, self.max_world_height as f64)
                .floor() as u32;

                for y in 0..self.chunk_size.y {
                    if position.y + y < height as u32 {
                        chunk.set_voxel(glam::UVec3::new(x, y, z), Voxel::Stone);
                    }
                }
            }
        }

        chunk
    }
}
