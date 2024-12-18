use super::validation::Validation;
use anyhow::Context;
use ash::vk::{self, PhysicalDevice, PhysicalDeviceType};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};

pub struct Renderer {
    entry: ash::Entry,
    instance: ash::Instance,
    physical_device: PhysicalDevice,
    device: ash::Device,
    surface: vk::SurfaceKHR,
    validation: Option<Validation>,
}

impl Renderer {
    pub fn new(debug: bool, window: &winit::window::Window) -> anyhow::Result<Self> {
        let entry = unsafe { ash::Entry::load() }?;
        let app_info = vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_3);
        let mut extensions: Vec<*const i8> =
            ash_window::enumerate_required_extensions(window.display_handle()?.as_raw())?.into();
        if debug {
            extensions.push(c"VK_EXT_debug_utils".as_ptr());
        }

        let validation_layer_name = [c"VK_LAYER_KHRONOS_validation".as_ptr()];
        let mut debug_create_info = Validation::desc();
        let instance = {
            let mut info = vk::InstanceCreateInfo::default()
                .application_info(&app_info)
                .enabled_extension_names(&extensions);
            if debug {
                info = info
                    .enabled_layer_names(&validation_layer_name)
                    .push_next(&mut debug_create_info);
            }

            unsafe { entry.create_instance(&info, None) }
        }?;

        let validation = if debug {
            Some(Validation::new(&entry, &instance)?)
        } else {
            None
        };

        let physical_device = choose_physical_device(&instance)?;

        let device = {
            let queue_info = [vk::DeviceQueueCreateInfo::default()
                .queue_family_index(select_queue_family(
                    &instance,
                    physical_device,
                    vk::QueueFlags::GRAPHICS,
                )?)
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

        let surface = unsafe {
            ash_window::create_surface(
                &entry,
                &instance,
                window.display_handle()?.as_raw(),
                window.window_handle()?.as_raw(),
                None,
            )
        }?;

        log::info!("Renderer has been successfull initialized");
        Ok(Self {
            entry,
            instance,
            physical_device,
            device,
            surface,
            validation,
        })
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
        .map(|(idx, __)| idx as u32);

    if index.is_none() {
        log::error!("Selected GPU does not support {:?}", flags);
    }

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
