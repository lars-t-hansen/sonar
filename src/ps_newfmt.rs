use crate::gpuapi;
use crate::output;
use crate::ps::{ProcInfo,PsOptions,SampleData};
use crate::systemapi;
use crate::util::three_places;

use std::collections::HashMap;

pub fn format_newfmt(
    c: &SampleData,
    system: &dyn systemapi::SystemAPI,
    opts: &PsOptions,
) -> output::Object {
    let mut envelope = output::newfmt_envelope(system);
    let (mut data, mut attrs) = output::newfmt_data(system, "sample");
    attrs.push_s("node", system.get_hostname());
    if opts.load {
        let mut sstate = output::Object::new();
        let mut cpu_load = output::Array::new();
        for v in &c.cpu_samples {
            cpu_load.push_i(*v as i64);
        }
        sstate.push_a("cpus", cpu_load);
        if let Some(gpu_samples) = &c.gpu_samples {
            let mut gpu_load = output::Array::new();
            for v in gpu_samples {
                gpu_load.push_o(format_newfmt_gpu_sample(v));
            }
            sstate.push_a("gpus", gpu_load);
        }
        attrs.push_o("system", sstate);
    }
    // Group processes under (user, jobid) except for jobid 0.
    // `collected` collects the sample indices for like (user,job) where job != 0.
    // `zeroes` collects the sample indices for job = 0.
    let mut collected = HashMap::<(&str,usize),Vec<usize>>::new();
    let mut zeroes = vec![];
    for i in 0..c.process_samples.len() {
        let sample = &c.process_samples[i];
        if sample.job_id == 0 {
            zeroes.push(i);
        } else {
            collected.
                entry((&sample.user, sample.job_id))
                .and_modify(|e| e.push(i))
                .or_insert(vec![i]);
        }
    }
    let mut jobs = output::Array::new();
    for k in zeroes {
        let j = &c.process_samples[k];
        jobs.push_o(format_newfmt_job(0, &j.user, &vec![k], &c.process_samples));
    }
    for ((user,id),ixs) in collected {
        jobs.push_o(format_newfmt_job(id, user, &ixs, &c.process_samples));
    }
    attrs.push_a("jobs", jobs);
    data.push_o("attributes", attrs);
    envelope.push_o("data", data);
    envelope
}

fn format_newfmt_gpu_sample(c: &gpuapi::CardState) -> output::Object {
    let mut s = output::Object::new();
    if c.device.index != 0 {
        s.push_i("index", c.device.index as i64);
    }
    if c.device.uuid != "" {
        s.push_s("uuid", c.device.uuid.clone());
    }
    // FIXME: "bad" - is it relevant at all?
    if c.fan_speed_pct != 0.0 {
        s.push_i("fan", c.fan_speed_pct.round() as i64);
    }
    if c.compute_mode != "" {
        s.push_s("mode", c.compute_mode.clone());
    }
    if c.perf_state != -1 {
        s.push_i("perf", c.perf_state);
    }
    if c.mem_used_kib != 0 {
        s.push_i("memory", c.mem_used_kib);
    }
    if c.gpu_utilization_pct != 0.0 {
        s.push_i("ce-util", c.gpu_utilization_pct.round() as i64);
    }
    if c.mem_utilization_pct != 0.0 {
        s.push_i("mem-util", c.mem_utilization_pct.round() as i64);
    }
    if c.temp_c != 0 {
        s.push_i("temp", c.temp_c as i64);
    }
    if c.power_watt != 0 {
        s.push_i("pow", c.power_watt as i64);
    }
    if c.power_limit_watt != 0 {
        s.push_i("power-limit", c.power_limit_watt as i64);
    }
    if c.ce_clock_mhz != 0 {
        s.push_i("ce-clock", c.ce_clock_mhz as i64);
    }
    if c.mem_clock_mhz != 0 {
        s.push_i("mem-clock", c.mem_clock_mhz as i64);
    }
    s
}

// Job and User are handled at a higher level, here we format the per-process fields that
// exclude those.

fn format_newfmt_job(
    id: usize,
    user: &str,
    ixs: &[usize],
    samples: &[ProcInfo],
) -> output::Object {
    let mut job = output::Object::new();
    job.push_u("job", id as u64);
    job.push_s("user", user.to_string());
    let mut procs = output::Array::new();
    for ix in ixs {
        procs.push_o(format_newfmt_sample(&samples[*ix]));
    }
    job.push_a("processes", procs);
    job
}

fn format_newfmt_sample(proc_info: &ProcInfo) -> output::Object {
    let mut fields = output::Object::new();

    if proc_info.rssanon_kib != 0 {
        fields.push_u("resident", proc_info.rssanon_kib as u64);
    }
    if proc_info.mem_size_kib != 0 {
        fields.push_u("virtual", proc_info.mem_size_kib as u64);
    }
    fields.push_s("cmd", proc_info.command.to_string());
    if proc_info.rolledup == 0 && proc_info.pid != 0 {
        // pid must be 0 for rolledup > 0 as there is no guarantee that there is any fixed
        // representative pid for a rolled-up set of processes: the set can change from run to run,
        // and sonar has no history.
        fields.push_u("pid", proc_info.pid as u64);
    }
    if proc_info.ppid != 0 {
        fields.push_u("ppid", proc_info.ppid as u64);
    }
    if proc_info.cpu_percentage != 0.0 {
        fields.push_f("cpu-avg", three_places(proc_info.cpu_percentage));
    }
    if proc_info.cputime_sec != 0 {
        fields.push_u("cpu-time", proc_info.cputime_sec as u64);
    }
    if proc_info.rolledup > 0 {
        fields.push_u("rolled-up", proc_info.rolledup as u64);
    }
    if !proc_info.gpus.is_empty() {
        let mut gpus = output::Array::new();
        for (_,g) in &proc_info.gpus {
            let mut gpu = output::Object::new();
            gpu.push_u("index", g.device.index as u64);
            gpu.push_s("uuid", g.device.uuid.clone());
            if g.gpu_util != 0.0 {
                gpu.push_f("gpu-util", g.gpu_util);
            }
            if g.gpu_mem != 0 {
                gpu.push_u("gpu-mem", g.gpu_mem);
            }
            if g.gpu_mem_util != 0.0 {
                gpu.push_f("gpu-mem-util", g.gpu_mem_util);
            }
            gpus.push_o(gpu);
        }
        fields.push_a("gpus", gpus);
    }

    fields
}
