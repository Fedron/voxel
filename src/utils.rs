pub fn coord_to_index(position: glam::UVec3, size: glam::UVec3) -> usize {
    position.x as usize
        + position.y as usize * size.x as usize
        + position.z as usize * size.x as usize * size.y as usize
}
