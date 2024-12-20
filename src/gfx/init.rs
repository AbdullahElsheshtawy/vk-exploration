use anyhow::Context;
use ash::{
    vk::{self, PhysicalDevice, PhysicalDeviceType},
    RawPtr,
};

pub fn cmd_buffer_begin_info(
    flags: vk::CommandBufferUsageFlags,
) -> vk::CommandBufferBeginInfo<'static> {
    vk::CommandBufferBeginInfo::default().flags(flags)
}

pub fn cmd_pool_create_info(
    queue_family_idx: u32,
    flags: vk::CommandPoolCreateFlags,
) -> vk::CommandPoolCreateInfo<'static> {
    vk::CommandPoolCreateInfo::default()
        .flags(flags)
        .queue_family_index(queue_family_idx)
}

pub fn cmd_buffer_allocate_info(
    pool: vk::CommandPool,
    count: u32,
) -> vk::CommandBufferAllocateInfo<'static> {
    vk::CommandBufferAllocateInfo::default()
        .command_pool(pool)
        .command_buffer_count(count)
        .level(vk::CommandBufferLevel::PRIMARY)
}

pub fn fence_create_info(flags: vk::FenceCreateFlags) -> vk::FenceCreateInfo<'static> {
    vk::FenceCreateInfo::default().flags(flags)
}

pub fn semaphore_create_info(flags: vk::SemaphoreCreateFlags) -> vk::SemaphoreCreateInfo<'static> {
    vk::SemaphoreCreateInfo::default().flags(flags)
}

pub fn select_queue_family(
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

pub fn choose_physical_device(instance: &ash::Instance) -> anyhow::Result<PhysicalDevice> {
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

pub fn image_subresource_range(aspect_mask: vk::ImageAspectFlags) -> vk::ImageSubresourceRange {
    vk::ImageSubresourceRange::default()
        .aspect_mask(aspect_mask)
        .level_count(vk::REMAINING_MIP_LEVELS)
        .layer_count(vk::REMAINING_ARRAY_LAYERS)
}

pub fn sem_submit_info(
    stage_mask: vk::PipelineStageFlags2,
    sem: vk::Semaphore,
) -> vk::SemaphoreSubmitInfo<'static> {
    vk::SemaphoreSubmitInfo::default()
        .semaphore(sem)
        .stage_mask(stage_mask)
        .value(1)
}

pub fn cmd_buffer_submit_info(cmd: vk::CommandBuffer) -> vk::CommandBufferSubmitInfo<'static> {
    vk::CommandBufferSubmitInfo::default().command_buffer(cmd)
}

pub fn submit_info<'a>(
    cmd: &'a vk::CommandBufferSubmitInfo<'a>,
    signal_sem: Option<&'a vk::SemaphoreSubmitInfo<'a>>,
    wait_sem: Option<&'a vk::SemaphoreSubmitInfo<'a>>,
) -> vk::SubmitInfo2<'a> {
    vk::SubmitInfo2 {
        wait_semaphore_info_count: if wait_sem.is_some() { 1 } else { 0 },
        p_wait_semaphore_infos: wait_sem.as_raw_ptr(),
        signal_semaphore_info_count: if signal_sem.is_some() { 1 } else { 0 },
        p_signal_semaphore_infos: signal_sem.as_raw_ptr(),
        command_buffer_info_count: 1,
        p_command_buffer_infos: cmd as *const vk::CommandBufferSubmitInfo,
        ..Default::default()
    }
}
