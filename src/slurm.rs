// Abstraction of jobs::JobManager for SLURM.

use crate::command;
use crate::jobs;
use crate::process;

pub struct SlurmJobManager {}

impl jobs::JobManager for SlurmJobManager {
    fn job_id_from_pid(&mut self, pid: usize, _processes: &[process::Process]) -> usize {
        let slurm_job_id = get_slurm_job_id(pid).unwrap_or_default();
        slurm_job_id.trim().parse::<usize>().unwrap_or_default()
    }

    fn need_process_tree(&self) -> bool {
        false
    }
}

fn get_slurm_job_id(pid: usize) -> Option<String> {
    let path = format!("/proc/{}/cgroup", pid);

    if !std::path::Path::new(&path).exists() {
        return None;
    }

    let command = format!(
        "cat /proc/{}/cgroup | grep -oP '(?<=job_).*?(?=/)' | head -n 1",
        pid
    );
    let timeout_seconds = 2;

    // TODO: Maybe propagate an error here?
    match command::safe_command(&command, timeout_seconds) {
        Ok(s) => Some(s),
        Err(_) => None,
    }
}
