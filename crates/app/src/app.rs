use std::{marker::PhantomData, time::Duration};

use anyhow::Result;
use egui::{Align2, ClippedPrimitive, FullOutput};
use egui_plot::Legend;
use vulkan::{
    ash::vk, AcquiredImage, CommandBuffer, CommandPool, Context, ContextBuilder, DeviceFeatures,
    ImageBarrier, RenderingAttachment, SemaphoreSubmitInfo, Swapchain, VERSION_1_3,
};
use winit::window::Window;

use crate::{
    camera::{Camera, Projection},
    gui::{Gui, GuiContext},
    utils::{create_command_buffers, create_storage_images},
    AppConfig, FrameStats, ImageAndView, InFlightFrames, StatsDisplayMode, IN_FLIGHT_FRAMES,
};

pub trait App: Sized {
    type Gui: Gui;

    fn new(base: &mut BaseApp<Self>) -> Result<Self>;

    fn update(
        &mut self,
        base: &mut BaseApp<Self>,
        image_index: usize,
        delta_time: Duration,
    ) -> Result<()>;

    fn record_raytracing_commands(
        &self,
        base: &BaseApp<Self>,
        buffer: &CommandBuffer,
        image_index: usize,
    ) -> Result<()>;

    fn record_raster_commands(&self, base: &BaseApp<Self>, image_index: usize) -> Result<()>;

    fn on_recreate_swapchain(&mut self, base: &BaseApp<Self>) -> Result<()>;
}

pub struct BaseApp<A: App> {
    phantom: PhantomData<A>,
    raytracing_enabled: bool,

    pub swapchain: Swapchain,
    pub command_pool: CommandPool,
    pub storage_images: Vec<ImageAndView>,
    pub command_buffers: Vec<CommandBuffer>,
    in_flight_frames: InFlightFrames,

    pub gui_context: GuiContext,
    pub(crate) stats_display_mode: StatsDisplayMode,
    pub context: Context,

    pub camera: Camera,
    pub projection: Projection,

    pub(crate) requested_swapchain_format: Option<vk::SurfaceFormatKHR>,
}

impl<A: App> BaseApp<A> {
    pub(crate) fn new(window: &Window, app_name: &str, app_config: AppConfig) -> Result<Self> {
        log::info!("Creating base application");

        let AppConfig {
            enable_raytracing,
            required_instance_extensions,
            enable_independent_blend,
        } = app_config;

        let mut required_extensions = vec!["VK_KHR_swapchain"];
        if enable_raytracing {
            required_extensions.push("VK_KHR_ray_tracing_pipeline");
            required_extensions.push("VK_KHR_acceleration_structure");
            required_extensions.push("VK_KHR_deferred_host_operations");
        }

        let mut context = ContextBuilder::new(window, window)
            .vulkan_version(VERSION_1_3)
            .app_name(app_name)
            .required_instance_extensions(required_instance_extensions)
            .required_device_extensions(&required_extensions)
            .required_device_features(DeviceFeatures {
                ray_tracing_pipeline: enable_raytracing,
                acceleration_structure: enable_raytracing,
                runtime_descriptor_array: enable_raytracing,
                buffer_device_address: enable_raytracing,
                dynamic_rendering: true,
                synchronization2: true,
                independent_blend: enable_independent_blend,
            })
            .with_raytracing_context(enable_raytracing)
            .build()?;

        let command_pool = context.create_command_pool(
            context.graphics_queue_family,
            Some(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
        )?;

        let swapchain = Swapchain::new(
            &context,
            window.inner_size().width,
            window.inner_size().height,
        )?;

        let storage_images = if enable_raytracing {
            create_storage_images(&mut context, swapchain.extent, swapchain.images.len())?
        } else {
            vec![]
        };

        let command_buffers = create_command_buffers(&command_pool, &swapchain)?;
        let in_flight_frames = InFlightFrames::new(&context, IN_FLIGHT_FRAMES)?;

        let camera = Camera::new(glam::Vec3::Z, -90.0_f32.to_radians(), 0.0);
        let projection = Projection::new(
            window.inner_size().width as f32 / window.inner_size().height as f32,
            60.0,
            0.1,
            1000.0,
        );

        let gui_context =
            GuiContext::new(&context, swapchain.format, window, IN_FLIGHT_FRAMES as _)?;

        Ok(Self {
            phantom: PhantomData,
            raytracing_enabled: enable_raytracing,

            command_pool,
            swapchain,
            storage_images,
            command_buffers,
            in_flight_frames,

            gui_context,
            stats_display_mode: StatsDisplayMode::Basic,
            context,

            camera,
            projection,

            requested_swapchain_format: None,
        })
    }

    pub(crate) fn recreate_swapchain(
        &mut self,
        width: u32,
        height: u32,
        format: Option<vk::SurfaceFormatKHR>,
    ) -> Result<()> {
        log::debug!("Recreating the swapchain");

        self.wait_for_gpu()?;

        self.swapchain
            .update(&self.context, width, height, format)?;

        if self.raytracing_enabled {
            let storage_images = create_storage_images(
                &mut self.context,
                self.swapchain.extent,
                self.swapchain.images.len(),
            )?;
            let _ = std::mem::replace(&mut self.storage_images, storage_images);
        }

        if let Some(format) = format {
            self.gui_context.update_framebuffer_params(format.format)?;
        }

        self.projection.resize(width as f32, height as f32);

        Ok(())
    }

    pub fn wait_for_gpu(&self) -> Result<()> {
        self.context.device_wait_idle()
    }

    pub(crate) fn draw(
        &mut self,
        window: &Window,
        base_app: &mut A,
        gui: &mut A::Gui,
        frame_stats: &mut FrameStats,
    ) -> Result<bool> {
        self.in_flight_frames.next();
        self.in_flight_frames.fence().wait(None)?;

        let gpu_time = (frame_stats.total_frame_count >= IN_FLIGHT_FRAMES)
            .then(|| self.in_flight_frames.gpu_frame_time_ms())
            .transpose()?
            .unwrap_or_default();
        frame_stats.set_gpu_time_time(gpu_time);
        frame_stats.tick();

        let next_image_result = self
            .swapchain
            .acquire_next_image(u64::MAX, self.in_flight_frames.image_available_semaphore());
        let image_index = match next_image_result {
            Ok(AcquiredImage { index, .. }) => index as usize,
            Err(err) => match err.downcast_ref::<vk::Result>() {
                Some(&vk::Result::ERROR_OUT_OF_DATE_KHR) => return Ok(true),
                _ => panic!("Error while acquiring next image: {}", err),
            },
        };
        self.in_flight_frames.fence().reset()?;

        if !self.in_flight_frames.gui_textures_to_free().is_empty() {
            self.gui_context
                .free_textures(&self.in_flight_frames.gui_textures_to_free())?;
        }

        let raw_input = self.gui_context.take_input(window);

        let FullOutput {
            platform_output,
            textures_delta,
            shapes,
            pixels_per_point,
            ..
        } = self.gui_context.run(raw_input, |ctx| {
            gui.build(ctx);
            self.build_performance_ui(ctx, frame_stats);
        });

        self.gui_context
            .handle_platform_output(window, platform_output);

        if !textures_delta.free.is_empty() {
            self.in_flight_frames
                .set_gui_textures_to_free(textures_delta.free);
        }

        if !textures_delta.set.is_empty() {
            self.gui_context
                .set_textures(
                    self.context.graphics_queue.inner,
                    self.context.command_pool.inner,
                    textures_delta.set.as_slice(),
                )
                .expect("failed to update texture");
        }

        let primitives = self.gui_context.tessellate(shapes, pixels_per_point);

        base_app.update(self, image_index, frame_stats.frame_time)?;

        self.record_command_buffer(image_index, base_app, pixels_per_point, &primitives)?;

        let command_buffer = &self.command_buffers[image_index];
        self.context.graphics_queue.submit(
            command_buffer,
            Some(SemaphoreSubmitInfo {
                semaphore: self.in_flight_frames.image_available_semaphore(),
                stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            }),
            Some(SemaphoreSubmitInfo {
                semaphore: self.in_flight_frames.render_finished_semaphore(),
                stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
            }),
            self.in_flight_frames.fence(),
        )?;

        let signal_semaphores = [self.in_flight_frames.render_finished_semaphore()];
        let present_result = self.swapchain.queue_present(
            image_index as _,
            &signal_semaphores,
            &self.context.present_queue,
        );
        match present_result {
            Ok(true) => return Ok(true),
            Err(err) => match err.downcast_ref::<vk::Result>() {
                Some(&vk::Result::ERROR_OUT_OF_DATE_KHR) => return Ok(true),
                _ => panic!("Failed to present queue: {}", err),
            },
            _ => {}
        }

        Ok(false)
    }

    fn record_command_buffer(
        &mut self,
        image_index: usize,
        base_app: &A,
        pixels_per_point: f32,
        primitives: &[ClippedPrimitive],
    ) -> Result<()> {
        self.command_buffers[image_index].reset()?;
        self.command_buffers[image_index].begin(None)?;
        self.command_buffers[image_index]
            .reset_all_timestamp_queries_from_pool(self.in_flight_frames.timing_query_pool());
        self.command_buffers[image_index].write_timestamp(
            vk::PipelineStageFlags2::NONE,
            self.in_flight_frames.timing_query_pool(),
            0,
        );

        if self.raytracing_enabled {
            base_app.record_raytracing_commands(
                self,
                &self.command_buffers[image_index],
                image_index,
            )?;
            let storage_image = &self.storage_images[image_index].image;

            self.command_buffers[image_index].pipeline_image_barriers(&[
                ImageBarrier {
                    image: &self.swapchain.images[image_index],
                    old_layout: vk::ImageLayout::UNDEFINED,
                    new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    src_access_mask: vk::AccessFlags2::empty(),
                    dst_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                    src_stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                    dst_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                },
                ImageBarrier {
                    image: storage_image,
                    old_layout: vk::ImageLayout::GENERAL,
                    new_layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    src_access_mask: vk::AccessFlags2::SHADER_WRITE,
                    dst_access_mask: vk::AccessFlags2::TRANSFER_READ,
                    src_stage_mask: vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR,
                    dst_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                },
            ]);

            self.command_buffers[image_index].copy_image(
                storage_image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                &self.swapchain.images[image_index],
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            );

            self.command_buffers[image_index].pipeline_image_barriers(&[
                ImageBarrier {
                    image: &self.swapchain.images[image_index],
                    old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    new_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    src_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                    dst_access_mask: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                    src_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                    dst_stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                },
                ImageBarrier {
                    image: storage_image,
                    old_layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    new_layout: vk::ImageLayout::GENERAL,
                    src_access_mask: vk::AccessFlags2::TRANSFER_READ,
                    dst_access_mask: vk::AccessFlags2::SHADER_WRITE,
                    src_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                    dst_stage_mask: vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR,
                },
            ]);
        } else {
            self.command_buffers[image_index].pipeline_image_barriers(&[ImageBarrier {
                image: &self.swapchain.images[image_index],
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                src_access_mask: vk::AccessFlags2::empty(),
                dst_access_mask: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                src_stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                dst_stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            }]);
        }

        base_app.record_raster_commands(self, image_index)?;

        self.command_buffers[image_index].begin_rendering(
            &[RenderingAttachment {
                view: &self.swapchain.views[image_index],
                load_op: vk::AttachmentLoadOp::DONT_CARE,
                clear_value: None,
            }],
            None,
            self.swapchain.extent,
        );

        self.gui_context.renderer.cmd_draw(
            self.command_buffers[image_index].inner,
            self.swapchain.extent,
            pixels_per_point,
            primitives,
        )?;

        self.command_buffers[image_index].end_rendering();

        self.command_buffers[image_index].pipeline_image_barriers(&[ImageBarrier {
            image: &self.swapchain.images[image_index],
            old_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            new_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            src_access_mask: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
            dst_access_mask: vk::AccessFlags2::empty(),
            src_stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            dst_stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
        }]);

        self.command_buffers[image_index].write_timestamp(
            vk::PipelineStageFlags2::TOP_OF_PIPE,
            self.in_flight_frames.timing_query_pool(),
            1,
        );

        self.command_buffers[image_index].end()?;

        Ok(())
    }

    fn build_performance_ui(&self, ctx: &egui::Context, frame_stats: &mut FrameStats) {
        if matches!(
            self.stats_display_mode,
            StatsDisplayMode::Basic | StatsDisplayMode::Full
        ) {
            egui::Window::new("Frame Stats")
                .anchor(Align2::RIGHT_TOP, [-5.0, 5.0])
                .collapsible(false)
                .interactable(false)
                .resizable(false)
                .drag_to_scroll(false)
                .min_size([150.0, 0.0])
                .max_size([150.0, 100.0])
                .show(ctx, |ui| {
                    ui.label(format!("{} FPS", frame_stats.fps_counter));
                    ui.separator();

                    ui.label(format!("Frame Time: {:.2?}", frame_stats.frame_time));
                    ui.label(format!("CPU Time: {:.2?}", frame_stats.cpu_time));
                    ui.label(format!("GPU Time: {:.2?}", frame_stats.gpu_time));
                });
        }

        if matches!(self.stats_display_mode, StatsDisplayMode::Full) {
            egui::TopBottomPanel::bottom("frametime_graphs").show(ctx, |ui| {
                ui.label("Frame Time (ms)");

                let frame_time: egui_plot::PlotPoints = frame_stats
                    .frame_time_ms_log
                    .0
                    .iter()
                    .enumerate()
                    .map(|(i, v)| [i as f64, *v as f64])
                    .collect();

                let cpu_time: egui_plot::PlotPoints = frame_stats
                    .cpu_time_ms_log
                    .0
                    .iter()
                    .enumerate()
                    .map(|(i, v)| [i as f64, *v as f64])
                    .collect();

                let gpu_time: egui_plot::PlotPoints = frame_stats
                    .cpu_time_ms_log
                    .0
                    .iter()
                    .enumerate()
                    .map(|(i, v)| [i as f64, *v as f64])
                    .collect();

                egui_plot::Plot::new("frame_time")
                    .height(80.0)
                    .allow_boxed_zoom(false)
                    .allow_double_click_reset(false)
                    .allow_drag(false)
                    .allow_scroll(false)
                    .allow_zoom(false)
                    .show_axes([false, true])
                    .legend(Legend::default())
                    .show(ui, |plot| {
                        plot.line(egui_plot::Line::new(frame_time).name("Frame Time"));
                        plot.line(egui_plot::Line::new(cpu_time).name("CPU Time"));
                        plot.line(egui_plot::Line::new(gpu_time).name("GPU Time"));
                    });
            });
        }
    }
}
