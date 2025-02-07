// Abstract system interface.

use crate::procfsapi;
use crate::gpu;
use crate::jobs;

pub trait SystemAPI {
    // The timestamp is invariant.
    fn get_timestamp(&self) -> String;

    // The host name is invariant.
    fn get_hostname(&self) -> String;

    fn get_procfs<'a>(&'a self) -> &'a dyn procfsapi::ProcfsAPI;

    fn get_gpuapi(&self) -> &dyn gpu::GpuAPI;

    fn get_jobs<'a>(&'a self) -> &'a dyn jobs::JobManager;
}
