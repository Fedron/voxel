use anyhow::Result;
use ash::{khr::surface, vk, Entry};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::{instance::Instance, utils::HandleError};

pub struct Surface {
    pub(crate) inner: surface::Instance,
    pub surface_khr: vk::SurfaceKHR,
}

impl Surface {
    pub(crate) fn new(
        entry: &Entry,
        instance: &Instance,
        window_handle: &dyn HasWindowHandle,
        display_handle: &dyn HasDisplayHandle,
    ) -> Result<Self> {
        let inner = surface::Instance::new(entry, &instance.inner);
        let surface_khr = unsafe {
            ash_window::create_surface(
                entry,
                &instance.inner,
                display_handle
                    .display_handle()
                    .map_err(|e| Into::<HandleError>::into(e))?
                    .as_raw(),
                window_handle
                    .window_handle()
                    .map_err(|e| Into::<HandleError>::into(e))?
                    .as_raw(),
                None,
            )?
        };

        Ok(Self { inner, surface_khr })
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.inner.destroy_surface(self.surface_khr, None);
        }
    }
}
