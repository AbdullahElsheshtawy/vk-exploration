use anyhow::Context;
use ash::vk::{self, PhysicalDevice, PhysicalDeviceType};
use winit::raw_window_handle::HasDisplayHandle;

use super::{surface::Surface, swapchain::Swapchain};

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
    frame_num: usize,
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

        let physical_device = choose_physical_device(&instance)?;

        let gfx_queue_family_idx =
            select_queue_family(&instance, physical_device, vk::QueueFlags::GRAPHICS)?;
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
            frame_num: 0,
        })
    }

    fn init_frame_data(
        device: &ash::Device,
        queue_family_idx: u32,
    ) -> anyhow::Result<[FrameData; FIF]> {
        let pool_info = command_pool_create_info(
            queue_family_idx,
            vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
        );
        let mut frames: [FrameData; FIF] = core::array::from_fn(|_| FrameData::default());
        (0..FIF).into_iter().for_each(|i| {
            let pool = unsafe { device.create_command_pool(&pool_info, None) }
                .expect(&format!("Could not Create command pool for FIF {}", i));
            let buffer =
                unsafe { device.allocate_command_buffers(&command_buffer_allocate_info(pool, 1)) }
                    .expect(&format!(
                        "Could not allocate command buffer/s for FIF {}",
                        i
                    ))[0];
            frames[i] = FrameData { pool, buffer }
        });

        Ok(frames)
    }

    pub fn current_frame(&self) -> &FrameData {
        &self.frames[self.frame_num % FIF]
    }
}

fn select_queue_family(
    instance: &ash::Instance,
    physical_device: PhysicalDevice,
    flags: vk::QueueFlags,
) -> anyhow::Result<u32> {
    let index = unsafe { instance.get_physical_device_queue_family_properties(physical_device) }
        .iter()
        .enumerate()
        .find(|(_, properties)| properties.queue_flags.contains(flags))
        .map(|(idx, _)| idx as u32);

    index.ok_or(anyhow::anyhow!(format!(
        "Selected GPU does not support {:?}",
        flags
    )))
}
fn choose_physical_device(instance: &ash::Instance) -> anyhow::Result<PhysicalDevice> {
    unsafe { instance.enumerate_physical_devices() }?
        .into_iter()
        .max_by_key(|physical_device| {
            match unsafe {
                instance
                    .get_physical_device_properties(*physical_device)
                    .device_type
            } {
                PhysicalDeviceType::DISCRETE_GPU => 100,
                PhysicalDeviceType::INTEGRATED_GPU => 75,
                PhysicalDeviceType::VIRTUAL_GPU => 50,
                PhysicalDeviceType::CPU => 25,
                PhysicalDeviceType::OTHER => 10,
                _ => 1,
            }
        })
        .context("No Graphics!")
}

impl Drop for Renderer {
    fn drop(&mut self) {
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

pub fn command_pool_create_info(
    queue_family_idx: u32,
    flags: vk::CommandPoolCreateFlags,
) -> vk::CommandPoolCreateInfo<'static> {
    vk::CommandPoolCreateInfo::default()
        .flags(flags)
        .queue_family_index(queue_family_idx)
}

pub fn command_buffer_allocate_info(
    pool: vk::CommandPool,
    count: u32,
) -> vk::CommandBufferAllocateInfo<'static> {
    vk::CommandBufferAllocateInfo::default()
        .command_pool(pool)
        .command_buffer_count(count)
        .level(vk::CommandBufferLevel::PRIMARY)
}

#[derive(Debug, Default)]
struct FrameData {
    pub pool: vk::CommandPool,
    pub buffer: vk::CommandBuffer,
}

impl FrameData {
    pub fn destroy(&self, device: &ash::Device) {
        unsafe { device.destroy_command_pool(self.pool, None) };
    }
}
