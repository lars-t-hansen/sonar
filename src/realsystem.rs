// Concrete system implementation.

use crate::hostname;
use crate::time;
use crate::procfsapi;
use crate::gpu;
use crate::realgpu;
use crate::systemapi;
use crate::jobs;

pub struct RealSystem {
    fs: procfsapi::RealFS,
    gpus: realgpu::RealGpu,
    jm: Option<Box<dyn jobs::JobManager>>,
}

impl RealSystem {
    pub fn new() -> RealSystem {
        RealSystem {
            fs: procfsapi::RealFS::new(),
            gpus: realgpu::RealGpu::new(),
            jm: None,
        }
    }

    pub fn with_jobmanager(self, jm: Box<dyn jobs::JobManager>) -> RealSystem {
        RealSystem {
            jm: Some(jm),
            ..self
        }
    }
}

impl systemapi::SystemAPI for RealSystem {
    fn get_timestamp(&self) -> String {
        // FIXME! Must cache this - interior mutability, or pass it as parameter during creation.
        //
        // Obtain the time stamp early so that it more properly reflects the time the sample was
        // obtained, not the time when reporting was allowed to run.  The latter is subject to greater
        // system effects, and using that timestamp increases the risk that the samples' timestamp order
        // improperly reflects the true order in which they were obtained.  See #100.
        time::now_iso8601()
    }

    fn get_hostname(&self) -> String {
        // Can cache this
        hostname::get()
    }

    fn get_procfs<'a>(&'a self) -> &'a dyn procfsapi::ProcfsAPI {
        &self.fs
    }

    fn get_gpuapi(&self) -> &dyn gpu::GpuAPI {
        &self.gpus
    }

    fn get_jobs<'a>(&'a self) -> &'a dyn jobs::JobManager {
        self.jm.as_ref().unwrap().as_ref()
    }
}
