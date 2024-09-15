use anyhow::Result;

use app::{App, AppConfig};

mod app;
mod camera;

fn main() -> Result<()> {
    app::run::<VoxelApp>(
        "Voxel",
        1920,
        1080,
        AppConfig {
            enable_raytracing: true,
            ..Default::default()
        },
    )
}

struct VoxelApp;

impl App for VoxelApp {
    fn new(base: &mut app::BaseApp<Self>) -> Result<Self> {
        let _ = base;

        Ok(VoxelApp)
    }

    fn update(
        &mut self,
        base: &mut app::BaseApp<Self>,
        image_index: usize,
        delta_time: std::time::Duration,
    ) -> Result<()> {
        let _ = base;
        let _ = image_index;
        let _ = delta_time;

        Ok(())
    }

    fn record_raytracing_commands(
        &self,
        base: &app::BaseApp<Self>,
        buffer: &vulkan::CommandBuffer,
        image_index: usize,
    ) -> Result<()> {
        let _ = base;
        let _ = buffer;
        let _ = image_index;

        Ok(())
    }

    fn record_raster_commands(&self, base: &app::BaseApp<Self>, image_index: usize) -> Result<()> {
        let _ = base;
        let _ = image_index;

        Ok(())
    }

    fn on_recreate_swapchain(&mut self, base: &app::BaseApp<Self>) -> Result<()> {
        let _ = base;

        Ok(())
    }
}
