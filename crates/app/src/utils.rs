use vulkan::{
    ash::vk, gpu_allocator::MemoryLocation, CommandBuffer, CommandPool, Context, ImageBarrier,
    Swapchain,
};

use crate::ImageAndView;

#[derive(Debug)]
pub struct Queue<T>(pub Vec<T>, usize);

impl<T> Queue<T> {
    pub fn new(max_size: usize) -> Self {
        Self(Vec::with_capacity(max_size), max_size)
    }

    pub fn push(&mut self, value: T) {
        if self.0.len() == self.1 {
            self.0.remove(0);
        }
        self.0.push(value);
    }
}

pub fn create_storage_images(
    context: &mut Context,
    extent: vk::Extent2D,
    count: usize,
) -> anyhow::Result<Vec<ImageAndView>> {
    let mut images = Vec::with_capacity(count);

    for _ in 0..count {
        let image = context.create_image(
            vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::STORAGE,
            MemoryLocation::GpuOnly,
            vk::Format::R8G8B8A8_UNORM,
            extent.width,
            extent.height,
        )?;

        let view = image.create_image_view(vk::ImageAspectFlags::COLOR)?;

        context.execute_one_time_commands(|cmd_buffer| {
            cmd_buffer.pipeline_image_barriers(&[ImageBarrier {
                image: &image,
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::GENERAL,
                src_access_mask: vk::AccessFlags2::NONE,
                dst_access_mask: vk::AccessFlags2::SHADER_WRITE,
                src_stage_mask: vk::PipelineStageFlags2::NONE,
                dst_stage_mask: vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR,
            }]);
        })?;

        images.push(ImageAndView { image, view })
    }

    Ok(images)
}

pub fn create_command_buffers(
    pool: &CommandPool,
    swapchain: &Swapchain,
) -> anyhow::Result<Vec<CommandBuffer>> {
    pool.allocate_command_buffers(vk::CommandBufferLevel::PRIMARY, swapchain.images.len() as _)
}
