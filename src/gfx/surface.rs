use ash::vk::{self, PhysicalDevice};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};

pub struct Surface {
    fns: ash::khr::surface::Instance,
    pub surface: vk::SurfaceKHR,
}
impl Surface {
    pub fn new(
        entry: &ash::Entry,
        instance: &ash::Instance,
        window: &winit::window::Window,
    ) -> anyhow::Result<Self> {
        let fns = ash::khr::surface::Instance::new(&entry, &instance);
        let surface = unsafe {
            ash_window::create_surface(
                &entry,
                &instance,
                window.display_handle()?.as_raw(),
                window.window_handle()?.as_raw(),
                None,
            )
        }?;

        Ok(Self { fns, surface })
    }

    pub fn get_caps(
        &self,
        physical_device: PhysicalDevice,
    ) -> anyhow::Result<vk::SurfaceCapabilitiesKHR> {
        unsafe {
            Ok(self
                .fns
                .get_physical_device_surface_capabilities(physical_device, self.surface)?)
        }
    }

    pub fn supported_present_modes(
        &self,
        physical_device: PhysicalDevice,
    ) -> anyhow::Result<Vec<vk::PresentModeKHR>> {
        Ok(unsafe {
            self.fns
                .get_physical_device_surface_present_modes(physical_device, self.surface)
        }?)
    }

    pub fn supported_formats(
        &self,
        physical_device: PhysicalDevice,
    ) -> anyhow::Result<Vec<vk::SurfaceFormatKHR>> {
        Ok(unsafe {
            self.fns
                .get_physical_device_surface_formats(physical_device, self.surface)
        }?)
    }

    pub fn destroy(&self) {
        unsafe { self.fns.destroy_surface(self.surface, None) };
    }
}

