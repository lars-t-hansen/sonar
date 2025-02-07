// MockFS is used for testing, it is instantiated with the values we want it to return.

#[cfg(test)]
use crate::procfsapi;

#[cfg(test)]
use std::collections::HashMap;

#[cfg(test)]
pub struct MockFS {
    files: HashMap<String, String>,
    pids: Vec<(usize, u32)>,
    users: HashMap<u32, String>,
    ticks_per_sec: usize,
    pagesz: usize,
    now: u64,
}

#[cfg(test)]
impl MockFS {
    pub fn new(
        files: HashMap<String, String>,
        pids: Vec<(usize, u32)>,
        users: HashMap<u32, String>,
        now: u64,
    ) -> MockFS {
        MockFS {
            files,
            pids,
            users,
            ticks_per_sec: 100,
            pagesz: 4,
            now,
        }
    }
}

#[cfg(test)]
impl procfsapi::ProcfsAPI for MockFS {
    fn read_to_string(&self, path: &str) -> Result<String, String> {
        match self.files.get(path) {
            Some(s) => Ok(s.clone()),
            None => Err(format!("Unable to read /proc/{path}")),
        }
    }

    fn read_proc_pids(&self) -> Result<Vec<(usize, u32)>, String> {
        Ok(self.pids.clone())
    }

    fn user_by_uid(&self, uid: u32) -> Option<String> {
        match self.users.get(&uid) {
            Some(s) => Some(s.clone()),
            None => None,
        }
    }

    fn clock_ticks_per_sec(&self) -> usize {
        self.ticks_per_sec
    }

    fn page_size_in_kib(&self) -> usize {
        self.pagesz
    }

    fn now_in_secs_since_epoch(&self) -> u64 {
        self.now
    }
}
