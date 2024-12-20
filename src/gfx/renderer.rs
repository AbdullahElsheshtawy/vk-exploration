use ash::vk::{self, PhysicalDevice};
use winit::raw_window_handle::HasDisplayHandle;

use super::{init, surface::Surface, swapchain::Swapchain, util};

const FIF: usize = 2;

pub struct Renderer {
    entry: ash::Entry,
    instance: ash::Instance,
    physical_device: PhysicalDevice,
    device: ash::Device,
    surface: Surface,
    swapchain: Swapchain,
    queue: vk::Queue,
    gfx_queue_family_idx: u32,
    frames: [FrameData; FIF],
    frame_counter: usize,
}

impl Renderer {
    pub fn new(window: &winit::window::Window) -> anyhow::Result<Self> {
        let entry = unsafe { ash::Entry::load() }?;
        let app_info = vk::ApplicationInfo::default()
            .application_name(c"Vulkan Exploration")
            .application_version(vk::make_api_version(0, 0, 1, 0))
            .engine_name(c"Vulkan Exploration Engine")
            .engine_version(vk::make_api_version(0, 0, 1, 0))
            .api_version(vk::API_VERSION_1_3);
        let required_extensions: Vec<*const i8> =
            ash_window::enumerate_required_extensions(window.display_handle()?.as_raw())?.into();

        let instance = {
            let info = vk::InstanceCreateInfo::default()
                .application_info(&app_info)
                .enabled_extension_names(&required_extensions);

            unsafe { entry.create_instance(&info, None) }
        }?;

        let physical_device = init::choose_physical_device(&instance)?;

        let gfx_queue_family_idx =
            init::select_queue_family(&instance, physical_device, vk::QueueFlags::GRAPHICS)?;
        let device = {
            let queue_info = [vk::DeviceQueueCreateInfo::default()
                .queue_family_index(gfx_queue_family_idx)
                .queue_priorities(&[1.0])];
            let extension_names = [vk::KHR_SWAPCHAIN_NAME.as_ptr()];
            let mut features_12 = vk::PhysicalDeviceVulkan12Features::default()
                .buffer_device_address(true)
                .descriptor_indexing(true);
            let mut features_13 = vk::PhysicalDeviceVulkan13Features::default()
                .dynamic_rendering(true)
                .synchronization2(true);
            let info = vk::DeviceCreateInfo::default()
                .queue_create_infos(&queue_info)
                .enabled_extension_names(&extension_names)
                .push_next(&mut features_12)
                .push_next(&mut features_13);

            unsafe { instance.create_device(physical_device, &info, None)? }
        };

        let surface = Surface::new(&entry, &instance, &window)?;

        let swapchain = Swapchain::new(
            &instance,
            &device,
            physical_device,
            &surface,
            window.inner_size().width,
            window.inner_size().height,
            None,
        )?;

        let queue = unsafe { device.get_device_queue(gfx_queue_family_idx, 0) };

        let frames = Self::init_frame_data(&device, gfx_queue_family_idx)?;

        Ok(Self {
            entry,
            instance,
            physical_device,
            device,
            surface,
            swapchain,
            queue,
            gfx_queue_family_idx,
            frames,
            frame_counter: 0,
        })
    }

    pub fn draw(&mut self) -> anyhow::Result<()> {
        unsafe {
            self.device
                .wait_for_fences(&[self.current_frame().render_fence], true, u64::MAX)?;
            self.device
                .reset_fences(&[self.current_frame().render_fence])?;
        }

        let (swapchain_image_idx, _) = unsafe {
            self.swapchain.fns.acquire_next_image(
                self.swapchain.swapchain,
                u64::MAX,
                self.current_frame().swapchain_sem,
                vk::Fence::null(),
            )
        }?;

        let cmd = self.current_frame().buffer;

        unsafe {
            self.device
                .reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty())?;
            self.device.begin_command_buffer(
                cmd,
                &init::cmd_buffer_begin_info(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?;
        }

        util::transition_image(
            &self.device,
            cmd,
            self.swapchain.images[swapchain_image_idx as usize],
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::GENERAL,
        );

        let flash = f32::abs(f32::sin(self.frame_counter as f32 / 120.0));
        let clear_value = vk::ClearColorValue {
            float32: [0.0, 0.0, flash, 1.0],
        };

        let clear_range = init::image_subresource_range(vk::ImageAspectFlags::COLOR);

        unsafe {
            self.device.cmd_clear_color_image(
                cmd,
                self.swapchain.images[swapchain_image_idx as usize],
                vk::ImageLayout::GENERAL,
                &clear_value,
                &[clear_range],
            )
        };

        util::transition_image(
            &self.device,
            cmd,
            self.swapchain.images[swapchain_image_idx as usize],
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::PRESENT_SRC_KHR,
        );

        unsafe { self.device.end_command_buffer(cmd) };

        // Prepare the submission to the queue
        // Wait on the present_sem as that semaphore is signaled when the swapchain is ready
        // Signal on the render_sem to signal That renderering has finished
        let cmd_info = init::cmd_buffer_submit_info(cmd);

        let wait_info = init::sem_submit_info(
            vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            self.current_frame().swapchain_sem,
        );
        let signal_info = init::sem_submit_info(
            vk::PipelineStageFlags2::ALL_GRAPHICS,
            self.current_frame().rendering_sem,
        );

        let submit_info = init::submit_info(&cmd_info, Some(&signal_info), Some(&wait_info));
        // Submit the command buffer and execute it
        // render_fence will now block until the graphic commands finish execution
        unsafe {
            self.device.queue_submit2(
                self.queue,
                &[submit_info],
                self.current_frame().render_fence,
            )?;
        }

        // Present
        // this will put the image just rendered into the visible window
        // Wait on the render_sem for that as its necessary that drawing commands have finished before the image is displayed

        let image_indices = [swapchain_image_idx];
        let wait_sems = [self.current_frame().rendering_sem];
        let swapchains = [self.swapchain.swapchain];
        let present_info = vk::PresentInfoKHR::default()
            .swapchains(&swapchains)
            .wait_semaphores(&wait_sems)
            .image_indices(&image_indices);

        unsafe {
            self.swapchain
                .fns
                .queue_present(self.queue, &present_info)?;
        }

        self.frame_counter += 1;
        Ok(())
    }

    fn init_frame_data(
        device: &ash::Device,
        queue_family_idx: u32,
    ) -> anyhow::Result<[FrameData; FIF]> {
        let mut frames: [FrameData; FIF] = core::array::from_fn(|_| FrameData::default());
        let pool_info = init::cmd_pool_create_info(
            queue_family_idx,
            vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
        );

        // Create the synchronization info
        // One fence to control when the gpu has finished rendering a frame
        // 2 semaphores to synchronize rendering with the swapchain
        // The fence has to start SIGNALED so it can be waited on for the first frame
        let fence_info = init::fence_create_info(vk::FenceCreateFlags::SIGNALED);
        let sem_info = init::semaphore_create_info(vk::SemaphoreCreateFlags::empty());

        for i in 0..FIF {
            let pool = unsafe { device.create_command_pool(&pool_info, None) }?;
            let buffer = unsafe {
                device.allocate_command_buffers(&init::cmd_buffer_allocate_info(pool, 1))
            }?[0];

            let render_fence = unsafe { device.create_fence(&fence_info, None) }?;
            let swapchain_sem = unsafe { device.create_semaphore(&sem_info, None) }?;
            let rendering_sem = unsafe { device.create_semaphore(&sem_info, None) }?;
            frames[i] = FrameData {
                pool,
                buffer,
                swapchain_sem,
                rendering_sem,
                render_fence,
            }
        }

        Ok(frames)
    }

    pub fn current_frame(&self) -> &FrameData {
        &self.frames[self.frame_counter % FIF]
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe { self.device.device_wait_idle() };
        // NOTE: swapchain MUST be destroyed before the surface
        for frame in &self.frames {
            frame.destroy(&self.device);
        }

        self.swapchain.destroy(&self.device);
        self.surface.destroy();

        unsafe { self.device.destroy_device(None) };
        unsafe { self.instance.destroy_instance(None) };
    }
}
#[derive(Debug, Default)]
pub struct FrameData {
    pub pool: vk::CommandPool,
    pub buffer: vk::CommandBuffer,
    pub swapchain_sem: vk::Semaphore,
    pub rendering_sem: vk::Semaphore,
    pub render_fence: vk::Fence,
}

impl FrameData {
    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_semaphore(self.swapchain_sem, None);
            device.destroy_semaphore(self.rendering_sem, None);
            device.destroy_fence(self.render_fence, None);
            device.destroy_command_pool(self.pool, None);
        }
    }
}
