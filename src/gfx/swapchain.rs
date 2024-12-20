use super::surface::Surface;
use ash::vk::{self, PhysicalDevice};

pub struct Swapchain {
    pub fns: ash::khr::swapchain::Device,
    pub swapchain: vk::SwapchainKHR,
    pub images: Vec<vk::Image>,
    pub views: Vec<vk::ImageView>,
    pub format: vk::Format,
    pub extent: vk::Extent2D,
}
impl Swapchain {
    pub fn new(
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: PhysicalDevice,
        surface: &Surface,
        width: u32,
        height: u32,
        old_swapchain: Option<Self>,
    ) -> anyhow::Result<Self> {
        let fns = ash::khr::swapchain::Device::new(instance, device);
        let extent = vk::Extent2D { width, height };
        let caps = surface.get_caps(physical_device)?;
        let format = {
            let surface_format = surface
                .supported_formats(physical_device)?
                .into_iter()
                .find(|surface_format| {
                    surface_format.format == vk::Format::B8G8R8A8_UNORM
                        && surface_format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
                });

            if surface_format.is_none() {
                return Err(anyhow::anyhow!("Desired Surface format not found!"));
            }

            surface_format.unwrap().format
        };

        let max_images = match caps.max_image_count {
            0 => u32::MAX,
            x => x,
        };
        let min_images = (caps.min_image_count).max(max_images);
        let mailbox_supported: bool = surface
            .supported_present_modes(physical_device)?
            .iter()
            .find(|&mode| *mode == vk::PresentModeKHR::MAILBOX)
            .is_some();
        let present_mode = if mailbox_supported {
            vk::PresentModeKHR::MAILBOX
        } else {
            vk::PresentModeKHR::FIFO
        };

        let mut info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface.surface)
            .min_image_count(min_images)
            .image_format(format)
            .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
            .image_array_layers(1)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .clipped(true)
            .image_extent(extent)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST)
            .present_mode(present_mode);

        if let Some(old_swapchain) = old_swapchain {
            info = info.old_swapchain(old_swapchain.swapchain);
        }

        let swapchain = unsafe { fns.create_swapchain(&info, None) }?;

        let images = unsafe { fns.get_swapchain_images(swapchain) }?;
        let views: Vec<vk::ImageView> = images
            .iter()
            .map(|image| {
                let view_info = vk::ImageViewCreateInfo::default()
                    .image(*image)
                    .format(format)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::R,
                        g: vk::ComponentSwizzle::G,
                        b: vk::ComponentSwizzle::B,
                        a: vk::ComponentSwizzle::A,
                    })
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .base_array_layer(0)
                            .level_count(1)
                            .layer_count(1),
                    );
                unsafe { device.create_image_view(&view_info, None) }.unwrap()
            })
            .collect();

        Ok(Self {
            fns,
            swapchain,
            images,
            views,
            format,
            extent,
        })
    }

    pub fn destroy(&self, device: &ash::Device) {
        self.views
            .iter()
            .for_each(|view| unsafe { device.destroy_image_view(*view, None) });
        unsafe { self.fns.destroy_swapchain(self.swapchain, None) };
    }
}
