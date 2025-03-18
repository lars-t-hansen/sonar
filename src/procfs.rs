/// Collect CPU process information without GPU information, from files in /proc.

#[cfg(test)]
use crate::mocksystem;
use crate::procfsapi;
use crate::systemapi;
#[cfg(test)]
use crate::systemapi::SystemAPI;

use std::collections::{HashMap, HashSet};

#[derive(PartialEq, Debug)]
pub struct Process {
    pub pid: usize,
    pub ppid: usize,
    pub pgrp: usize,
    pub uid: usize,
    pub user: String, // _noinfo_<uid> if name unobtainable
    pub cpu_pct: f64,
    pub mem_pct: f64,
    pub cputime_sec: usize,
    pub mem_size_kib: usize,
    pub rssanon_kib: usize,
    pub command: String,
    pub has_children: bool,
}

// All figures in KB, as reported by OS in /proc/meminfo.

pub struct Memory {
    pub total: u64,
    pub available: u64,
}

// Read the /proc/meminfo file from the fs and return the value for total installed memory.

pub fn get_memory(fs: &dyn procfsapi::ProcfsAPI) -> Result<Memory, String> {
    let mut memory = Memory { total: 0, available: 0 };
    let meminfo_s = fs.read_to_string("meminfo")?;
    {
        let total = &mut memory.total;
        let available = &mut memory.available;
        for l in meminfo_s.split('\n') {
            let ptr : &mut u64;
            if l.starts_with("MemTotal: ") {
                ptr = total;
            } else if l.starts_with("MemAvailable: ") {
                ptr = available;
            } else {
                continue;
            }
            // We expect "tag:\s+(\d+)\s+kB", roughly
            let fields = l.split_ascii_whitespace().collect::<Vec<&str>>();
            if fields.len() != 3 || fields[2] != "kB" {
                return Err(format!("Unexpected {} in /proc/meminfo: {l}", fields[0]));
            }
            *ptr = parse_usize_field(&fields, 1, l, "meminfo", 0, fields[0])? as u64;
        }
    }
    if memory.total == 0 {
        return Err(format!(
            "Could not find MemTotal in /proc/meminfo: {meminfo_s}"
        ));
    }
    Ok(memory)
}

/// Read the /proc/cpuinfo file from the fs and return information about installed CPUs.
///
/// Fun fact: this file is very different on x86_64 and aarch64 and requires custom parsing for
/// each.

pub struct CpuInfo {
    pub sockets: i32,
    pub cores_per_socket: i32,
    pub threads_per_core: i32,
    pub cores: Vec<CoreInfo>,
}

#[allow(dead_code)]
pub struct CoreInfo {
    pub model_name: String,
    pub logical_index: i32,
    pub physical_index: i32,
}

#[cfg(target_arch = "x86_64")]
pub fn get_cpu_info(fs: &dyn procfsapi::ProcfsAPI) -> Result<CpuInfo, String> {
    get_cpu_info_x86_64(fs)
}

#[cfg(target_arch = "aarch64")]
pub fn get_cpu_info(fs: &dyn procfsapi::ProcfsAPI) -> Result<CpuInfo, String> {
    get_cpu_info_aarch64(fs)
}

// Otherwise, get_cpu_info() will be undefined and we'll get a compile error; rustc is good about
// notifying the user that there are ifdef'd cases that are inactive.

#[cfg(any(target_arch = "x86_64", test))]
pub fn get_cpu_info_x86_64(fs: &dyn procfsapi::ProcfsAPI) -> Result<CpuInfo, String> {
    let mut physids = HashSet::new();
    let mut cores = vec![];
    let mut model_name = None;
    let mut physical_index = 0i32;
    let mut logical_index = 0i32;
    let mut cores_per_socket = 0i32;
    let mut siblings = 0i32;
    let mut sockets = 0i32;

    // On x86_64, the first line of every blob is "processor", which carries the logical index, and
    // all other lines relate to that.  Probably in principle, the "siblings" and "cpu cores" fields
    // could be different among sockets but we ignore that.

    let cpuinfo = fs.read_to_string("cpuinfo")?;
    for l in cpuinfo.split('\n') {
        if l.starts_with("processor") {
            if let Some(model_name) = model_name {
                cores.push(CoreInfo {
                    model_name,
                    physical_index,
                    logical_index,
                })
            }
            model_name = None;
            logical_index = i32_field(l)?;
            physical_index = 0i32;
        } else if l.starts_with("model name") {
            model_name = Some(text_field(l)?);
        } else if l.starts_with("physical id") {
            physical_index = i32_field(l)?;
            if !physids.contains(&physical_index) {
                physids.insert(physical_index);
                sockets += 1;
            }
        } else if l.starts_with("siblings") {
            siblings = i32_field(l)?;
        } else if l.starts_with("cpu cores") {
            cores_per_socket = i32_field(l)?;
        }
    }
    if let Some(model_name) = model_name {
        cores.push(CoreInfo {
            model_name,
            physical_index,
            logical_index,
        })
    }
    if cores.len() == 0 || sockets == 0 || siblings == 0 || cores_per_socket == 0 {
        return Err("Incomplete information in /proc/cpuinfo".to_string());
    }
    let threads_per_core = siblings / cores_per_socket;
    Ok(CpuInfo { sockets, cores_per_socket, threads_per_core, cores })
}

#[cfg(any(target_arch = "aarch64", test))]
pub fn get_cpu_info_aarch64(fs: &dyn procfsapi::ProcfsAPI) -> Result<CpuInfo, String> {
    let mut processors = HashSet::<i32>::new();
    let mut model_major = 0i32;
    let mut model_minor = 0i32;

    // Tested on UiO "freebio3" node.  The first line of every blob is `processor`, which carries
    // the logical index.  There is no separate physical index.  Indeed the values on freebio3 seem
    // to be pretty borked, e.g. BogoMIPS = 50.00 is nuts.

    let cpuinfo = fs.read_to_string("cpuinfo")?;
    for l in cpuinfo.split('\n') {
        if l.starts_with("processor") {
            processors.insert(i32_field(l)?);
        } else if l.starts_with("CPU architecture") {
            model_major = i32_field(l)?;
        } else if l.starts_with("CPU variant") {
            model_minor = i32_field(l)?;
        }
    }

    let cores_per_socket = processors.len() as i32;
    let threads_per_core = 1;
    let sockets = 1;
    let model_name = format!("ARMv{model_major}.{model_minor}");
    let mut cores = vec![];
    for core in 0..sockets*cores_per_socket {
        cores.push(CoreInfo {
            logical_index: core,
            physical_index: 0,
            model_name: model_name.clone(),
        })
    }
    Ok(CpuInfo { sockets, cores_per_socket, threads_per_core, cores })
}

#[cfg(any(target_arch = "x86_64", test))]
fn text_field(l: &str) -> Result<String, String> {
    if let Some((_, after)) = l.split_once(':') {
        Ok(after.trim().to_string())
    } else {
        Err(format!("Missing text field in {l}"))
    }
}

fn i32_field(l: &str) -> Result<i32, String> {
    if let Some((_, after)) = l.split_once(':') {
        let after = after.trim();
        match after.strip_prefix("0x") {
            Some(s) => match i32::from_str_radix(s, 16) {
                Ok(n) => Ok(n),
                Err(_) => Err(format!("Bad int field {l}")),
            },
            None => match after.parse::<i32>() {
                Ok(n) => Ok(n),
                Err(_) => Err(format!("Bad int field {l}")),
            },
        }
    } else {
        Err(format!("Missing or bad int field in {l}"))
    }
}

pub fn get_boot_time(fs: &dyn procfsapi::ProcfsAPI) -> Result<u64, String> {
    let stat_s = fs.read_to_string("stat")?;
    for l in stat_s.split('\n') {
        if l.starts_with("btime ") {
            let fields = l.split_ascii_whitespace().collect::<Vec<&str>>();
            return Ok(parse_usize_field(&fields, 1, l, "stat", 0, "btime")? as u64);
        }
    }
    Err(format!("Could not find btime in /proc/stat: {stat_s}"))
}

/// Obtain process information via /proc and return a hashmap of structures with all the information
/// we need, keyed by pid.  Pids uniquely tag the records.
///
/// This returns Ok(data) on success, otherwise Err(msg).
///
/// This function uniformly uses /proc, even though in some cases there are system calls that
/// provide the same information.

pub fn get_process_information(
    system: &dyn systemapi::SystemAPI,
    memtotal_kib: usize,
) -> Result<(HashMap<usize, Process>, u64, Vec<u64>), String> {
    // We need this for a lot of things.  On x86 and x64 this is always 100 but in principle it
    // might be something else, so read the true value.

    let ticks_per_sec = system.get_clock_ticks_per_sec() as u64;
    if ticks_per_sec == 0 {
        return Err("Could not get a sensible CLK_TCK".to_string());
    }

    let fs = system.get_procfs();

    // Extract system data from /proc/stat.  https://man7.org/linux/man-pages/man5/procfs.5.html.
    //
    // The per-CPU usage is the sum of some fields of the `cpuN` lines.  These are in ticks since
    // boot.  In addition there is an across-the-system line called simply `cpu` with the same
    // format.  These data are useful for analyzing core bindings.
    //
    // The boot time is first field of the `btime` line of /proc/stat.  It is measured in seconds
    // since epoch.  We need this to compute the process's real time, which we need to compute
    // ps-compatible cpu utilization.

    let mut boot_time = 0;
    let mut cpu_total_secs = 0;
    let mut per_cpu_secs = vec![];
    let stat_s = fs.read_to_string("stat")?;
    for l in stat_s.split('\n') {
        if l.starts_with("cpu") {
            // Based on sysstat sources, the "nice" time is not included in the "user" time.  (But
            // guest times, which we ignore here, are included in their overall times.)  And
            // irq/softirq numbers can be a substantial fraction of "system" time.  So sum user,
            // nice, sys, irq, and softirq as a sensible proxy for time spent on "work" on the CPU.
            const STAT_FIELDS: [usize; 5] = [1, 2, 3, 6, 7];

            let fields = l.split_ascii_whitespace().collect::<Vec<&str>>();
            let mut sum = 0;
            for i in STAT_FIELDS {
                sum += parse_usize_field(&fields, i, l, "stat", 0, "cpu")? as u64;
            }
            if l.starts_with("cpu ") {
                cpu_total_secs = sum / ticks_per_sec;
            } else {
                let cpu_no = match fields[0][3..].parse::<usize>() {
                    Ok(x) => x,
                    Err(_) => continue, // Too harsh to error out
                };
                if per_cpu_secs.len() < cpu_no + 1 {
                    per_cpu_secs.resize(cpu_no + 1, 0u64);
                }
                per_cpu_secs[cpu_no] = sum / ticks_per_sec;
            }
        } else if l.starts_with("btime ") {
            let fields = l.split_ascii_whitespace().collect::<Vec<&str>>();
            boot_time = parse_usize_field(&fields, 1, l, "stat", 0, "btime")? as u64;
        }
    }
    if boot_time == 0 {
        return Err(format!("Could not find btime in /proc/stat: {stat_s}"));
    }

    // Enumerate all pids, and collect the uids while we're here.
    //
    // Just ignore dirents that cause trouble, there wouldn't normally be any in proc, but if there
    // are we probably don't care.  We assume that sonar has sufficient permissions to inspect all
    // "interesting" processes.
    //
    // Note that a pid may disappear between the time we see it here and the time we get around to
    // reading it, later, and that new pids may appear meanwhile.  We should ignore both issues.

    let pids = fs.read_proc_pids()?;

    // Collect remaining system data from /proc/{pid}/stat for the enumerated pids.

    let kib_per_page = system.get_page_size_in_kib();
    let mut result = HashMap::<usize, Process>::new();
    let mut ppids = HashSet::<usize>::new();
    let mut user_table = UserTable::new();
    let clock_ticks_per_sec = ticks_per_sec as f64;

    for (pid, uid) in pids {
        // Basic system variables.  Intermediate time values are represented in ticks to prevent
        // various roundoff artifacts resulting in NaN or Infinity.

        let bsdtime_ticks;
        let mut realtime_ticks;
        let ppid;
        let pgrp;
        let mut comm;
        let utime_ticks;
        let stime_ticks;
        if let Ok(line) = fs.read_to_string(&format!("{pid}/stat")) {
            // The comm field is a little tricky, it must be extracted first as the contents between
            // the first '(' and the last ')' in the line.
            let commstart = line.find('(');
            let commend = line.rfind(')');
            let field_storage: String;
            let fields: Vec<&str>;
            match (commstart, commend) {
                (None, _) | (_, None) => {
                    return Err(format!(
                        "Could not parse command from /proc/{pid}/stat: {line}"
                    ));
                }
                (Some(commstart), Some(commend)) => {
                    comm = line[commstart + 1..commend].to_string();
                    field_storage = line[commend + 1..].trim().to_string();
                    fields = field_storage
                        .split_ascii_whitespace()
                        .collect::<Vec<&str>>();
                }
            };

            // NOTE relative to the `proc` documentation: All field offsets in the following are
            // relative to the command (so ppid is 2, not 4), and then they are zero-based, not
            // 1-based (so ppid is actually 1).

            // Fields[0] is the state.  These characters are relevant for modern kernels:
            //  R running
            //  S sleeping in interruptible wait
            //  D sleeping in uninterruptible disk wait
            //  Z zombie
            //  T stopped on a signal
            //  t stopped for tracing
            //  X dead
            //
            // For some of these, the fields may take on different values, in particular, for X and
            // Z it is known that some of the values we want are represented as -1 and parsing them
            // as unsigned will fail.  For Z it also looks like some of the fields could have
            // surprising zero values; one has to be careful when dividing.
            //
            // In particular for Z, field 5 "tpgid" has been observed to be -1.  For X, many of the
            // fields are -1.

            // Zombie jobs cannot be ignored, because they are indicative of system health and the
            // information about their presence is used in consumers.

            let dead = fields[0] == "X";
            let zombie = fields[0] == "Z";

            if dead {
                // Just drop dead jobs
                continue;
            }

            if zombie {
                // This tag is used by consumers but it's an artifact of `ps`, not the kernel
                comm += " <defunct>";
            }

            ppid = parse_usize_field(&fields, 1, &line, "stat", pid, "ppid")?;
            pgrp = parse_usize_field(&fields, 2, &line, "stat", pid, "pgrp")?;

            // Generally we want to record cumulative self+child time.  The child time we read will
            // be for children that have terminated and have been wait()ed for.  The logic is that
            // in a tree of processes in a job, Sonar will observe the parent and the children
            // independently and their respective (cumulative) cpu times, and then when the children
            // exit the time they spent will not be lost but will be accounted for as child time in
            // the parent, and the parent will later be observed by Sonar again.  In addition,
            // children that are relatively short-lived and not observed by Sonar at all are
            // accounted for in this manner.  The result is that the observed CPU time in a process
            // tree is pretty accurate (except that we lose some data between the last observation
            // and root process termination).
            //
            // However, there is a problem if jobs can be nested within the process tree and we want
            // to account jobs separately.  The root process of a subjob is the child of some
            // process in the parent job.  When the root of the subjob exits, its time is propagated
            // up to the parent job, which therefore can sometimes appear to have used an impossible
            // amount of CPU time in a very short time.  Sonar *cannot* correct for this problem: it
            // has no history, and has no notion of a job or process being "gone".  Instead, enough
            // data must be emitted by Sonar for a postprocessor of the data to reconstruct the job
            // tree and correct the data, if necessary.
            utime_ticks = parse_usize_field(&fields, 11, &line, "stat", pid, "utime")? as f64;
            stime_ticks = parse_usize_field(&fields, 12, &line, "stat", pid, "stime")? as f64;
            let cutime_ticks = parse_usize_field(&fields, 13, &line, "stat", pid, "cutime")? as f64;
            let cstime_ticks = parse_usize_field(&fields, 14, &line, "stat", pid, "cstime")? as f64;
            bsdtime_ticks = utime_ticks + stime_ticks + cutime_ticks + cstime_ticks;
            let start_time_ticks =
                parse_usize_field(&fields, 19, &line, "stat", pid, "starttime")? as f64;

            // boot_time and the current time are both time_t, ie, a 31-bit quantity in 2023 and a
            // 32-bit quantity before 2038.  clock_ticks_per_sec is on the order of 100.  Ergo
            // boot_ticks and now_ticks can be represented in about 32+7=39 bits, fine for an f64.
            let now_ticks = system.get_now_in_secs_since_epoch() as f64 * clock_ticks_per_sec;
            let boot_ticks = boot_time as f64 * clock_ticks_per_sec;

            // start_time_ticks should be on the order of a few years, there is no risk of overflow
            // here, and in any case boot_ticks + start_time_ticks <= now_ticks, and by the above
            // reasoning now_ticks fits in an f64, ergo the sum does too.
            //
            // Take the max with 1 here to ensure realtime_ticks is not zero.
            realtime_ticks = now_ticks - (boot_ticks + start_time_ticks);
            if realtime_ticks < 1.0 {
                realtime_ticks = 1.0;
            }
        } else {
            // This is *usually* benign - the process may have gone away since we enumerated the
            // /proc directory.  It is *possibly* indicative of a permission problem, but that
            // problem would be so pervasive that diagnosing it here is not right.
            continue;
        }

        // We want the value corresponding to the "size" field printed by ps.  This is a saga.  When
        // ps prints "size", it is with the pr_swappable() function of procps-ng (flagged variously
        // in the source as "Ugly old Debian thing" and "SCO"). This prints VM_DATA + VM_STACK,
        // scaled to KiB.  The values for VM_DATA and VM_STACK are obtained from /proc/{pid}/status
        // (not /proc/{pid}/stat).  However the man page for /proc says that those values are
        // inaccurate and that one should instead use /proc/{pid}/statm.  In that file, we want the
        // "data" field which is documented as "data + stack", this is the sixth space-separated
        // field.

        let size_kib;
        let rss_kib;
        if let Ok(s) = fs.read_to_string(&format!("{pid}/statm")) {
            let fields = s.split_ascii_whitespace().collect::<Vec<&str>>();
            rss_kib = parse_usize_field(&fields, 1, &s, "statm", pid, "resident set size")?
                * kib_per_page;
            size_kib = parse_usize_field(&fields, 5, &s, "statm", pid, "data size")? * kib_per_page;
        } else {
            // This is *usually* benign - see above.
            continue;
        }

        // The best value for resident memory is probably the Pss (proportional set size) field of
        // /proc/{pid}/smaps_rollup, see discussion on bug #126.  But that field is privileged.
        //
        // A contender is RssAnon of /proc/{pid}/status, which corresponds to "private data".  It
        // does not include text or file mappings, though these actually also take up real memory.
        // But text can be shared, which matters both when we roll up processes and when the program
        // is executing multiple times on the system, and file mappings, when they exist, are
        // frequently read-only and evictable.
        //
        // RssAnon will tend to be lower than Pss.  For typical scientific codes, discounting text
        // and read-only file mappings is probably more or less OK since private data will dwarf
        // these.  The main source of inaccuracy is resident read/write file mappings.
        //
        // In order to not confuse the matter we're going to name the fields in our internal data
        // structures and in the output by the fields that they are taken from, so "rssanon", not
        // "resident" or "rss" or similar.
        let mut rssanon_kib = 0;
        let mut was_found = false;
        if let Ok(status_info) = fs.read_to_string(&format!("{pid}/status")) {
            was_found = true;
            for l in status_info.split('\n') {
                if l.starts_with("RssAnon:") {
                    // We expect "RssAnon:\s+(\d+)\s+kB", roughly; there may be tabs.
                    let fields = l.split_ascii_whitespace().collect::<Vec<&str>>();
                    if fields.len() != 3 || fields[2] != "kB" {
                        return Err(format!("Unexpected RssAnon in /proc/{pid}/status: {l}"));
                    }
                    rssanon_kib = parse_usize_field(
                        &fields,
                        1,
                        l,
                        "status",
                        pid,
                        "private resident set size",
                    )?;
                    break;
                }
            }
        }
        if rssanon_kib == 0 {
            // This is *usually* benign - see above.  But in addition, kernel threads and processes
            // appear not to have the RssAnon field in /proc/{pid}/status.  In the interest of not
            // filtering too much too early, we'll just keep going here with a zero value if the
            // file was found but was missing that field.
            if !was_found {
                continue;
            }
        }

        // Now compute some derived quantities.

        // pcpu and pmem are rounded to ##.#.  We're going to get slightly different answers here
        // than ps because we use float arithmetic; frequently this code will produce values that
        // are one-tenth of a percent off from those of ps.  One can argue about whether round(),
        // floor() or ceil() is the most correct, but it's unlikely to matter much.

        // realtime_ticks is nonzero, so this division will not produce NaN or Infinity
        let pcpu_value = (utime_ticks + stime_ticks) / realtime_ticks;
        let pcpu_formatted = (pcpu_value * 1000.0).round() / 10.0;

        // clock_ticks_per_sec is nonzero, so this division will not produce NaN or Infinity.  See
        // block comment earlier about why bsdtime_ticks is the best base value here.
        let cputime_sec = (bsdtime_ticks / clock_ticks_per_sec).round() as usize;

        // Note ps uses rss not size here.  Also, ps doesn't trust rss to be <= 100% of memory, so
        // let's not trust it either.  memtotal_kib is nonzero, so this division will not produce
        // NaN or Infinity.
        let pmem = f64::min(
            ((rss_kib as f64) * 1000.0 / (memtotal_kib as f64)).round() / 10.0,
            99.9,
        );

        result.insert(
            pid,
            Process {
                pid,
                ppid,
                pgrp,
                uid: uid as usize,
                user: user_table.lookup(system, uid),
                cpu_pct: pcpu_formatted,
                mem_pct: pmem,
                cputime_sec,
                mem_size_kib: size_kib,
                rssanon_kib,
                command: comm,
                has_children: false,
            },
        );
        ppids.insert(ppid);
    }

    // Mark the processes that have children.
    for (_, p) in result.iter_mut() {
        p.has_children = ppids.contains(&p.pid);
    }

    Ok((result, cpu_total_secs, per_cpu_secs))
}

fn parse_usize_field(
    fields: &[&str],
    ix: usize,
    line: &str,
    file: &str,
    pid: usize,
    fieldname: &str,
) -> Result<usize, String> {
    if ix >= fields.len() {
        if pid == 0 {
            return Err(format!("Index out of range for /proc/{file}: {ix}: {line}"));
        } else {
            return Err(format!(
                "Index out of range for /proc/{pid}/{file}: {ix}: {line}"
            ));
        }
    }
    if let Ok(n) = fields[ix].parse::<usize>() {
        return Ok(n);
    }
    if pid == 0 {
        Err(format!(
            "Could not parse {fieldname} in /proc/{file}: {line}"
        ))
    } else {
        Err(format!(
            "Could not parse {fieldname} from /proc/{pid}/{file}: {line}"
        ))
    }
}

// The UserTable optimizes uid -> name lookup.

struct UserTable {
    ht: HashMap<u32, String>,
}

impl UserTable {
    fn new() -> UserTable {
        UserTable { ht: HashMap::new() }
    }

    fn lookup(&mut self, system: &dyn systemapi::SystemAPI, uid: u32) -> String {
        if let Some(name) = self.ht.get(&uid) {
            name.clone()
        } else if let Some(name) = system.user_by_uid(uid) {
            self.ht.insert(uid, name.clone());
            name
        } else {
            format!("_user_{uid}")
        }
    }
}

// For the parse test we use the full text of stat and meminfo, but for stat we only want the
// 'btime' line and for meminfo we only want the 'MemTotal:' line.  Other tests can economize on the
// input.

#[test]
pub fn procfs_parse_test() {
    let pids = vec![(4018, 1000)];

    let mut users = HashMap::new();
    users.insert(1000, "zappa".to_string());

    let mut files = HashMap::new();
    files.insert("stat".to_string(), std::include_str!("testdata/stat.txt").to_string());
    files.insert("meminfo".to_string(), std::include_str!("testdata/meminfo.txt").to_string());
    files.insert(
        "4018/stat".to_string(),
        "4018 (firefox) S 2190 2189 2189 0 -1 4194560 19293188 3117638 1823 557 51361 15728 5390 2925 20 0 187 0 16400 5144358912 184775 18446744073709551615 94466859782144 94466860597976 140720852341888 0 0 0 0 4096 17663 0 0 0 17 4 0 0 0 0 0 94466860605280 94466860610840 94466863497216 140720852350777 140720852350820 140720852350820 140720852357069 0".to_string());
    files.insert(
        "4018/statm".to_string(),
        "1255967 185959 54972 200 0 316078 0".to_string(),
    );
    files.insert("4018/status".to_string(), "RssAnon: 12345 kB".to_string());

    let ticks_per_sec = 100.0; // We define this
    let utime_ticks = 51361.0; // field(/proc/4018/stat, 14)
    let stime_ticks = 15728.0; // field(/proc/4018/stat, 15)
    let boot_time = 1698303295.0; // field(/proc/stat, "btime")
    let start_ticks = 16400.0; // field(/proc/4018/stat, 22)
    let rss: f64 = 185959.0 * 4.0; // pages_to_kib(field(/proc/4018/statm, 1))
    let memtotal = 16093776.0; // field(/proc/meminfo, "MemTotal:")
    let size = 316078 * 4; // pages_to_kib(field(/proc/4018/statm, 5))
    let rssanon = 12345; // field(/proc/4018/status, "RssAnon:")

    // now = boot_time + start_time + utime_ticks + stime_ticks + arbitrary idle time
    let now = (boot_time
        + (start_ticks / ticks_per_sec)
        + (utime_ticks / ticks_per_sec)
        + (stime_ticks / ticks_per_sec)
        + 2000.0) as u64;

    let system = mocksystem::MockSystem::new()
        .with_files(files)
        .with_pids(pids)
        .with_users(users)
        .with_now(now)
        .freeze();
    let fs = system.get_procfs();
    let memory = get_memory(fs).expect("Test: Must have data");
    assert!(memory.total == 16093776);
    assert!(memory.available == 8162068);
    let (mut info, total_secs, per_cpu_secs) =
        get_process_information(&system, memory.total as usize).expect("Test: Must have data");
    assert!(info.len() == 1);
    let mut xs = info.drain();
    let p = xs.next().expect("Test: Should have data").1;
    assert!(p.pid == 4018); // from enumeration of /proc
    assert!(p.uid == 1000); // ditto
    assert!(p.user == "zappa"); // from getent
    assert!(p.command == "firefox"); // field(/proc/4018/stat, 2)
    assert!(p.ppid == 2190); // field(/proc/4018/stat, 4)
    assert!(p.pgrp == 2189); // field(/proc/4018/stat, 5)

    let now_time = now as f64;
    let now_ticks = now_time * ticks_per_sec;
    let boot_ticks = boot_time * ticks_per_sec;
    let realtime_ticks = now_ticks - (boot_ticks + start_ticks);
    let cpu_pct_value = (utime_ticks + stime_ticks) / realtime_ticks;
    let cpu_pct = (cpu_pct_value * 1000.0).round() / 10.0;
    assert!(p.cpu_pct == cpu_pct);

    let mem_pct = (rss * 1000.0 / memtotal).round() / 10.0;
    assert!(p.mem_pct == mem_pct);

    assert!(p.mem_size_kib == size);
    assert!(p.rssanon_kib == rssanon);

    assert!(total_secs == (241155 + 582 + 127006 + 0 + 3816) / 100); // "cpu " line of "stat" data
    assert!(per_cpu_secs.len() == 8);
    assert!(per_cpu_secs[0] == (32528 + 189 + 19573 + 0 + 1149) / 100); // "cpu0 " line of "stat" data
    assert!(per_cpu_secs[7] == (27582 + 61 + 12558 + 0 + 426) / 100); // "cpu7 " line of "stat" data
}

#[test]
pub fn procfs_dead_and_undead_test() {
    let pids = vec![(4018, 1000), (4019, 1000), (4020, 1000)];

    let mut users = HashMap::new();
    users.insert(1000, "zappa".to_string());

    let mut files = HashMap::new();
    files.insert("stat".to_string(), "btime 1698303295".to_string());
    files.insert(
        "meminfo".to_string(),
        "MemTotal:       16093776 kB".to_string(),
    );
    files.insert(
        "4018/stat".to_string(),
        "4018 (firefox) S 2190 2189 2189 0 -1 4194560 19293188 3117638 1823 557 51361 15728 5390 2925 20 0 187 0 16400 5144358912 184775 18446744073709551615 94466859782144 94466860597976 140720852341888 0 0 0 0 4096 17663 0 0 0 17 4 0 0 0 0 0 94466860605280 94466860610840 94466863497216 140720852350777 140720852350820 140720852350820 140720852357069 0".to_string());
    files.insert(
        "4019/stat".to_string(),
        "4019 (firefox) Z 2190 2189 2189 0 -1 4194560 19293188 3117638 1823 557 51361 15728 5390 2925 20 0 187 0 16400 5144358912 184775 18446744073709551615 94466859782144 94466860597976 140720852341888 0 0 0 0 4096 17663 0 0 0 17 4 0 0 0 0 0 94466860605280 94466860610840 94466863497216 140720852350777 140720852350820 140720852350820 140720852357069 0".to_string());
    files.insert(
        "4020/stat".to_string(),
        "4020 (python3) X 0 -1 -1 0 -1 4243524 0 0 0 0 0 0 0 0 20 0 0 0 10643829 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 17 3 0 0 0 0 0 0 0 0 0 0 0 0 0".to_string());

    files.insert(
        "4018/statm".to_string(),
        "1255967 185959 54972 200 0 316078 0".to_string(),
    );
    files.insert(
        "4019/statm".to_string(),
        "1255967 185959 54972 200 0 316078 0".to_string(),
    );
    files.insert(
        "4020/statm".to_string(),
        "1255967 185959 54972 200 0 316078 0".to_string(),
    );
    files.insert("4018/status".to_string(), "RssAnon: 12345 kB".to_string());
    files.insert("4019/status".to_string(), "RssAnon: 12345 kB".to_string());
    files.insert("4020/status".to_string(), "RssAnon: 12345 kB".to_string());

    let system = mocksystem::MockSystem::new()
        .with_files(files)
        .with_pids(pids)
        .with_users(users)
        .freeze();
    let fs = system.get_procfs();
    let memory = get_memory(fs).expect("Test: Must have data");
    let (mut info, _, _) = get_process_information(&system, memory.total as usize).expect("Test: Must have data");

    // 4020 should be dropped - it's dead
    assert!(info.len() == 2);

    let mut xs = info.drain();
    let mut p = xs.next().expect("Test: Should have some data").1;
    let mut q = xs.next().expect("Test: Should have more data").1;
    if p.pid > q.pid {
        (p, q) = (q, p);
    }
    assert!(p.pid == 4018);
    assert!(p.command == "firefox");
    assert!(q.pid == 4019);
    assert!(q.command == "firefox <defunct>");
}

#[test]
pub fn procfs_cpuinfo_test_x86_64() {
    let mut files = HashMap::new();
    files.insert("cpuinfo".to_string(), std::include_str!("testdata/cpuinfo-x86_64.txt").to_string());
    let system = mocksystem::MockSystem::new().with_files(files).freeze();
    let CpuInfo { sockets, cores_per_socket, threads_per_core, cores } =
        get_cpu_info_x86_64(system.get_procfs()).expect("Test: Must have data");
    assert!(cores[0].model_name.find("E5-2637").is_some());
    assert!(sockets == 2);
    assert!(cores_per_socket == 4);
    assert!(threads_per_core == 2);
}

#[test]
pub fn procfs_cpuinfo_test_aarch64() {
    let mut files = HashMap::new();
    files.insert("cpuinfo".to_string(), std::include_str!("testdata/cpuinfo-aarch64.txt").to_string());
    let system = mocksystem::MockSystem::new().with_files(files).freeze();
    let CpuInfo { sockets, cores_per_socket, threads_per_core, cores } =
        get_cpu_info_aarch64(system.get_procfs()).expect("Test: Must have data");
    assert!(cores[0].model_name.find("ARMv8.3").is_some());
    assert!(sockets == 1);
    assert!(cores_per_socket == 96);
    assert!(threads_per_core == 1);
}
