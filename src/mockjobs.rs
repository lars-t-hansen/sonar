#[cfg(test)]
use crate::jobs;

#[cfg(test)]
use crate::procfs;

#[cfg(test)]
use std::collections::HashMap;

#[cfg(test)]
pub struct MockJobManager { }

#[cfg(test)]
impl jobs::JobManager for MockJobManager {
    fn job_id_from_pid(&self, pid: usize, _processes: &HashMap<usize, procfs::Process>)
        -> usize {
        pid
    }
}

