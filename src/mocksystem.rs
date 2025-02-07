use crate::systemapi;
use crate::mockfs;
use crate::procfsapi;
use crate::gpu;
use crate::jobs;

use std::collections::HashMap;
use std::cell::{Cell,RefCell};

// Test system

#[derive(Default)]
pub struct MockSystem {
    // Installed by with_jobmanager
    jm: Option<Box<dyn jobs::JobManager>>,

    // Installed by with_hostname
    hostname: Option<String>,

    // files, pids, users, now installed variously and then consumed when the fs is gotten
    files: RefCell<Option<HashMap<String, String>>>,
    pids: RefCell<Option<Vec<(usize, u32)>>>,
    users: RefCell<Option<HashMap<u32, String>>>,
    now: RefCell<Option<u64>>,
    fs: RefCell<Option<RefCell<mockfs::MockFS>>>,
}

/* These should be defaults
    let files = HashMap::new();
    let pids = vec![];
    let users = HashMap::new();
    let now = procfsapi::unix_now();

Timestamp
        "2025-01-24 09:19:00+01:00",

Then the FS is created like this
        &procfsapi::MockFS::new(files, pids, users, now),

And the GPU API like this
        &gpu::MockGpu::new(),

Basically when we call get_procfs(), the procfs must be constructed with the
extant values and saved in the cell.  The complication is that we don't have
a default value for that, so there's an option.
 */

impl MockSystem {
    pub fn new() -> MockSystem {
        MockSystem {
            ..Default::default()
        }
    }

    pub fn with_hostname(self, hostname: String) -> MockSystem {
        MockSystem {
            hostname: Some(hostname),
            ..self
        }
    }

    pub fn with_jobmanager(self, jm: Box<dyn jobs::JobManager>) -> MockSystem {
        MockSystem {
            jm: Some(jm),
            ..self
        }
    }

    pub fn with_files(self, files: HashMap<String, String>) -> MockSystem {
        self.files.replace(Some(files));
        self
    }

    pub fn with_users(self, users: HashMap<u32, String>) -> MockSystem {
        self.users.replace(Some(users));
        self
    }

    pub fn with_pids(self, pids: Vec<(usize, u32)>) -> MockSystem {
        self.pids.replace(Some(pids));
        self
    }

    pub fn with_time(self, now: u64) -> MockSystem {
        self.now.replace(Some(now));
        self
    }
}

impl systemapi::SystemAPI for MockSystem {
    fn get_timestamp(&self) -> String {
        todo!()
    }

    fn get_hostname(&self) -> String {
        match &self.hostname {
            Some(hn) => hn.clone(),
            None => "default.host".to_string(),
        }
    }

    // This doesn't compile, presumably we need Box<dyn ...> again, what a disaster

    fn get_procfs<'a>(&'a self) -> &'a dyn procfsapi::ProcfsAPI {
        if self.fs.borrow().is_none() {
            let files = self.files.take();
            let pids = self.pids.take();
            let users = self.users.take();
            let now = self.now.take();
            self.fs.replace(Some(RefCell::new(
                mockfs::MockFS::new(files.unwrap(),
                                    pids.unwrap(),
                                    users.unwrap(),
                                    now.unwrap()))));
        }
        self.fs.borrow().as_ref().unwrap().borrow().as_ref()
    }

    fn get_gpuapi(&self) -> &dyn gpu::GpuAPI {
        todo!()
    }

    fn get_jobs<'a>(&'a self) -> &'a dyn jobs::JobManager {
        self.jm.as_ref().unwrap().as_ref()
    }
}
