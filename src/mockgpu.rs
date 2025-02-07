use crate::gpu::{GPU, GpuAPI};

#[cfg(test)]
pub struct MockGpu {}

#[cfg(test)]
impl MockGpu {
    pub fn new() -> MockGpu {
        MockGpu {}
    }
}

#[cfg(test)]
impl GpuAPI for MockGpu {
    fn probe(&self) -> Option<Box<dyn GPU>> {
        None
    }
}
