// Get info about AMD graphics cards by parsing the output of rocm-smi.
//
// This is pretty hacky!  Something better than this is likely needed and hopefully possible.

use crate::command::{self, CmdError};
use crate::gpu;
use crate::ps::UserTable;
use crate::TIMEOUT_SECONDS;

use std::cmp::Ordering;
use std::path::Path;

#[cfg(test)]
use crate::util::map;

pub struct AmdGPU {}

pub fn probe() -> Option<Box<dyn gpu::GPU>> {
    if amd_present() {
        Some(Box::new(AmdGPU {}))
    } else {
        None
    }
}

impl gpu::GPU for AmdGPU {
    fn get_manufacturer(&mut self) -> String {
        "AMD".to_string()
    }

    fn get_card_configuration(&mut self) -> Result<Vec<gpu::Card>, String> {
        get_amd_configuration()
    }

    fn get_process_utilization(
        &mut self,
        user_by_pid: &UserTable,
    ) -> Result<Vec<gpu::Process>, String> {
        get_amd_utilization(user_by_pid)
    }

    fn get_card_utilization(&mut self) -> Result<Vec<gpu::CardState>, String> {
        Ok(vec![])
    }
}

// On all nodes we've looked at (ML systems, Lumi), /sys/module/amdgpu exists iff there are AMD
// accelerators present.

fn amd_present() -> bool {
    Path::new("/sys/module/amdgpu").exists()
}

// We only have one machine with AMD GPUs at UiO and rocm-smi is unable to show eg how much memory
// is installed on each card on this machine, so this is pretty limited.  But we are at least able
// to extract gross information about the installed cards.
//
// `rocm-smi --showproductname` lists the cards.  The "Card series" line has the card number and
// model name.  There is no memory information, so record it as zero.
//
// TODO: It may be possible to find memory sizes using lspci.  Run `lspci -v` and capture the
// output.  Now look for the line "Kernel modules: amdgpu".  The lines that are part of that block
// of info will have a couple of `Memory at ... ` lines that have memory block sizes, and the first
// line of the info block will have the GPU model.  The largest memory block size is likely the one
// we want.
//
// (It does not appear that the lspci trick works with the nvidia cards - the memory block sizes are
// too small.  This is presumably all driver dependent.)

fn get_amd_configuration() -> Result<Vec<gpu::Card>, String> {
    match command::safe_command("rocm-smi", &["--showproductname"], TIMEOUT_SECONDS) {
        Ok(raw_text) => {
            let mut cards = vec![];
            for l in raw_text.lines() {
                // We want to match /^GPU\[(\d+)\].*Card series:\s*(.*)$/ but we really only care
                // about \2, which is the description.
                //
                // Newer rocm-smi switched from "Card series" to "Card Series", sigh.
                if l.starts_with("GPU[") {
                    if let Some((_, after)) = l.split_once("Card series:") {
                        cards.push(gpu::Card {
                            model: after.trim().to_string(),
                            ..Default::default()
                        });
                    } else if let Some((_, after)) = l.split_once("Card Series:") {
                        cards.push(gpu::Card {
                            model: after.trim().to_string(),
                            ..Default::default()
                        });
                    }
                }
            }
            Ok(cards)
        }
        Err(CmdError::CouldNotStart(_)) => Ok(vec![]),
        Err(e) => Err(format!("{:?}", e)),
    }
}

// Get information about AMD cards.
//
// Err(e) really means the command started running but failed, for the reason given.  If the
// command could not be found or no card is present, we return Ok(vec![]).

fn get_amd_utilization(user_by_pid: &UserTable) -> Result<Vec<gpu::Process>, String> {
    // I've not been able to combine the two invocations of rocm-smi yet; we have to run the command
    // twice.  Not a happy situation.

    Ok(extract_amd_information(
        &get_raw_per_device_info()?,
        &get_raw_per_pid_info()?,
        user_by_pid,
    ))
}

// Put it all together from the command output.

fn extract_amd_information(
    per_device_info: &[(f64, f64)],
    per_pid_info: &[(usize, Vec<usize>)],
    user_by_pid: &UserTable,
) -> Vec<gpu::Process> {
    let mut num_processes_per_device = vec![0; per_device_info.len()];
    per_pid_info.iter().for_each(|(_, devs)| {
        devs.iter()
            .for_each(|dev| num_processes_per_device[*dev] += 1)
    });
    let mut processes = vec![];
    // The utilization for one process on one device is the total utilization for the device
    // divided by the number of processes using the device.
    per_pid_info.iter().for_each(|(pid, devs)| {
        devs.iter().for_each(|dev| {
            let (user, uid) = if let Some((user, uid)) = user_by_pid.get(pid) {
                (user.to_string(), *uid)
            } else {
                ("_zombie_".to_owned() + &pid.to_string(), gpu::ZOMBIE_UID)
            };
            processes.push(gpu::Process {
                device: Some(*dev),
                pid: *pid,
                user,
                uid,
                gpu_pct: per_device_info[*dev].0 / num_processes_per_device[*dev] as f64,
                mem_pct: per_device_info[*dev].1 / num_processes_per_device[*dev] as f64,
                mem_size_kib: 0,
                command: "_noinfo_".to_string(),
            })
        })
    });
    processes.sort_by(|p, q| {
        let fst = p.device.cmp(&q.device);
        if fst == Ordering::Equal {
            p.pid.cmp(&q.pid)
        } else {
            fst
        }
    });
    processes
}

// Return a dense map from device index starting at zero to gpu and gpumem utilization; empty if
// rocm-smi is not present; or an error if rocm-smi failed.
//
// Unfortunately the output format of rocm-smi is not stable.  So first we try to parse the CSV
// form, subsequently we try the old format.

fn get_raw_per_device_info() -> Result<Vec<(f64, f64)>, String> {
    match command::safe_command(
        "rocm-smi",
        &["--showuse", "--showmemuse", "--csv"],
        TIMEOUT_SECONDS,
    ) {
        Ok(text) => {
            if let Ok(info) = parse_csv_concise_command(&text) {
                return Ok(info);
            }
            // Otherwise fall through to second attempt below
        }
        Err(CmdError::CouldNotStart(_)) => {
            return Ok(vec![]);
        }
        Err(_) => {
            // Fall through, we're going to assume this is some problem with the command line
            // switches, the next attempt will hopefully surface any problems with the cards
            // themselves.
        }
    }
    match command::safe_command("rocm-smi", &[], TIMEOUT_SECONDS) {
        Ok(text) => parse_text_concise_command(&text),
        Err(CmdError::CouldNotStart(_)) => Ok(vec![]),
        Err(e) => Err(format!("{:?}", e)),
    }
}

#[cfg(test)]
macro_rules! proc(
    { $a:expr, $b:expr, $c:expr, $d:expr, $e: expr, $f: expr } => {
        gpu::Process { device: $a,
                       pid: $b,
                       user: $c.to_string(),
                       uid: $d,
                       gpu_pct: $e,
                       mem_pct: $f,
                       mem_size_kib: 0,
                       command: "_noinfo_".to_string()
        }
    });

#[test]
fn test_extract_amd_information() {
    let concise = "
================================= Concise Info =================================
GPU  Temp (DieEdge)  AvgPwr  SCLK     MCLK    Fan     Perf  PwrCap  VRAM%  GPU%
0    53.0c           220.0W  1576Mhz  945Mhz  10.98%  auto  220.0W   57%   99%
1    26.0c           3.0W    852Mhz   167Mhz  9.41%   auto  220.0W    5%   63%
================================================================================
";
    let pidgpu = "
============================= GPUs Indexed by PID ==============================
PID 28156 is using 2 DRM device(s):
0 1
PID 28154 is using 1 DRM device(s):
0
================================================================================
";
    let users = map! {
    28156 => ("bob", 1001usize)
    };
    let zs = extract_amd_information(
        &parse_text_concise_command(concise).expect("Test: AMD text concise information"),
        &parse_showpidgpus_command(pidgpu).expect("Test: AMD pid gpu information"),
        &users);
    assert!(zs.eq(&vec![
        proc! { Some(0), 28154, "_zombie_28154", gpu::ZOMBIE_UID, 99.0/2.0, 57.0/2.0 },
        proc! { Some(0), 28156, "bob", 1001, 99.0/2.0, 57.0/2.0 },
        proc! { Some(1), 28156, "bob", 1001, 63.0, 5.0 },
    ]));
}

// The format here is line-oriented:
//
// There should initially be at least one line with at least three fields which should
// start with these strings in order (with this capitalization, sigh):
//    device
//    GPU use
//    GPU Memory
//
// Subsequently there should be lines starting with "cardN,", these are are information for that
// card.
//
// All other lines are junk.

fn parse_csv_concise_command(raw_text: &str) -> Result<Vec<(f64, f64)>, String> {
    let lines = raw_text.lines().collect::<Vec<&str>>();
    let mut mappings = vec![];
    let mut found_device = false;
    for l in lines {
        if l.starts_with("device") {
            if found_device {
                return Err("Inconsistent output".to_string());
            }
            let fields = l.split(',').collect::<Vec<&str>>();
            if fields.len() >= 3
                && fields[1].starts_with("GPU use")
                && fields[2].starts_with("GPU Memory")
            {
                found_device = true;
            }
        } else if let Some(rest) = l.strip_prefix("card") {
            let fields = rest.split(',').collect::<Vec<&str>>();
            let mut dev = None;
            let mut gpu = None;
            let mut gpumem = None;
            if fields.len() >= 3 {
                dev = match fields[0].parse::<usize>() {
                    Ok(n) => Some(n),
                    _ => None,
                };
                gpu = match fields[1].parse::<f64>() {
                    Ok(n) => Some(n),
                    _ => None,
                };
                gpumem = match fields[2].parse::<f64>() {
                    Ok(n) => Some(n),
                    _ => None,
                };
            }
            match (dev, gpu, gpumem) {
                (Some(dev), Some(gpu), Some(gpumem)) => {
                    if mappings.len() < dev + 1 {
                        mappings.resize(dev + 1, (0.0, 0.0))
                    }
                    mappings[dev] = (gpu, gpumem);
                }
                _ => {}
            }
        }
    }
    if found_device && mappings.len() > 0 {
        Ok(mappings)
    } else {
        Err("Inconsistent output".to_string())
    }
}

// Return a vector of AMD GPU utilization indexed by device number: (gpu%, mem%)
//
// The gpu% has been verified to be roughly instantaneous utilization, as it would be for
// `rocm-smi -u`, see the manual page.  That is, this is not some (long) running average.
//
// The mem% is instantaneous memory utilization as expected.

fn parse_text_concise_command(raw_text: &str) -> Result<Vec<(f64, f64)>, String> {
    let block = find_block(raw_text, "= Concise Info =");
    if block.len() > 1 {
        let hdr = block[0].split_whitespace().collect::<Vec<&str>>();
        if hdr[hdr.len() - 2] == "VRAM%" && hdr[hdr.len() - 1] == "GPU%" {
            let mut i = 1;
            let mut mappings = vec![];
            while i < block.len() {
                let fields = block[i].split_whitespace().collect::<Vec<&str>>();
                let dev = fields[0].parse::<usize>().unwrap_or_default();
                // The fields should have the format N% and if they don't we always
                // default to 0.
                let mem = fields[fields.len() - 2]
                    .strip_suffix('%')
                    .unwrap_or("0")
                    .parse::<f64>()
                    .unwrap_or_default();
                let gpu = fields[fields.len() - 1]
                    .strip_suffix('%')
                    .unwrap_or("0")
                    .parse::<f64>()
                    .unwrap_or_default();
                if mappings.len() < dev + 1 {
                    mappings.resize(dev + 1, (0.0, 0.0))
                }
                mappings[dev] = (gpu, mem);
                i += 1;
            }
            Ok(mappings)
        } else {
            Err("Unexpected `Concise Info` header in output for AMD card:\n".to_string() + raw_text)
        }
    } else {
        Err("`Concise Info` block not found in output for AMD card:\n".to_string() + raw_text)
    }
}

#[test]
fn test_parse_text_concise_command() {
    let xs = parse_text_concise_command(
        "
================================= Concise Info =================================
GPU  Temp (DieEdge)  AvgPwr  SCLK     MCLK    Fan     Perf  PwrCap  VRAM%  GPU%
0    53.0c           220.0W  1576Mhz  945Mhz  10.98%  auto  220.0W   57%   99%
1    26.0c           3.0W    852Mhz   167Mhz  9.41%   auto  220.0W    5%   63%
================================================================================
",
    )
    .expect("Test: Must have data");
    assert!(xs.eq(&vec![(99.0, 57.0), (63.0, 5.0)]));
}

#[test]
fn test_parse_csv_concise_command() {
    let xs = parse_csv_concise_command(
        "
device,GPU use (%),GPU Memory Allocated (VRAM%),Memory Activity
card0,99,57,N/A
card1,63,5,N/A
",
    )
    .unwrap();
    assert!(xs.eq(&vec![(99.0, 57.0), (63.0, 5.0)]));
}

// Return a sparse map from pid to devices used by the pid; empty if rocm-smi is not present; or an
// error if rocm-smi failed.

fn get_raw_per_pid_info() -> Result<Vec<(usize, Vec<usize>)>, String> {
    match command::safe_command("rocm-smi", &["--showpidgpus"], TIMEOUT_SECONDS) {
        Ok(showpidgpus_raw_text) => parse_showpidgpus_command(&showpidgpus_raw_text),
        Err(CmdError::CouldNotStart(_)) => Ok(vec![]),
        Err(e) => Err(format!("{:?}", e)),
    }
}

// Return a vector of (PID, DEVICES) where DEVICES is a vector of the devices used by the PID.  The
// PID is a string, the devices are numbers.  See test cases below for the various forms
// expected/supported.
//
// The PIDs are unique, ie, the return value is technically a function.

fn parse_showpidgpus_command(raw_text: &str) -> Result<Vec<(usize, Vec<usize>)>, String> {
    let block = find_block(raw_text, "= GPUs Indexed by PID =");
    if block.len() == 1 && block[0].starts_with("No KFD PIDs") {
        // No processes running.
        Ok(vec![])
    } else if block.len() > 1 && block.len() % 2 == 0 {
        let mut mappings = vec![];
        let mut i = 0;
        while i < block.len() {
            let xs = block[i].split_whitespace().collect::<Vec<&str>>();
            if xs[0] == "PID" && xs[2] == "is" && xs[3] == "using" && xs[5] == "DRM" {
                let pid = xs[1].parse::<usize>().unwrap_or_default();
                let numdev = xs[4].parse::<usize>().unwrap_or_default();
                let devices = if numdev > 0 {
                    block[i + 1]
                        .split_whitespace()
                        .map(|d| d.parse::<usize>().unwrap_or_default())
                        .collect::<Vec<usize>>()
                } else {
                    vec![]
                };
                mappings.push((pid, devices))
            }
            i += 2;
        }
        Ok(mappings)
    } else {
        Err("`GPUs Indexed By PID` block not found in output for AMD card\n".to_string() + raw_text)
    }
}

// TODO: Multiple processes on a single device

#[test]
fn test_parse_showpidgpus_command() {
    let xs = parse_showpidgpus_command(
        "
============================= GPUs Indexed by PID ==============================
PID 25774 is using 1 DRM device(s):
0
================================================================================
",
    )
    .expect("Test: Must have data");
    assert!(xs.eq(&vec![(25774, vec![0])]));
    let xs = parse_showpidgpus_command(
        "
============================= GPUs Indexed by PID ==============================
No KFD PIDs currently running
================================================================================
",
    )
    .expect("Test: Must have data");
    assert!(xs.eq(&vec![]));

    let xs = parse_showpidgpus_command(
        "
============================= GPUs Indexed by PID ==============================
PID 28156 is using 1 DRM device(s):
1
PID 28154 is using 1 DRM device(s):
0
================================================================================
",
    )
    .expect("Test: Must have data");
    assert!(xs.eq(&vec![(28156, vec![1]), (28154, vec![0])]));
    let xs = parse_showpidgpus_command(
        "
============================= GPUs Indexed by PID ==============================
PID 29212 is using 2 DRM device(s):
0 1
================================================================================
",
    )
    .expect("Test: Must have data");
    assert!(xs.eq(&vec![(29212, vec![0, 1])]));
}

// Grab the first block of rocm-smi output we see that contains the trigger string, and return the
// lines within that block.

fn find_block<'a>(raw_text: &'a str, trigger: &str) -> Vec<&'a str> {
    let lines = raw_text.lines().collect::<Vec<&str>>();
    let mut i = 0;
    let mut b = vec![];
    while i < lines.len() && !lines[i].contains(trigger) {
        i += 1;
    }
    if i < lines.len() && lines[i].contains(trigger) {
        i += 1;
        while i < lines.len() && !is_terminator(lines[i]) {
            b.push(lines[i]);
            i += 1;
        }
    }
    b
}

fn is_terminator(s: &str) -> bool {
    s.chars().all(|c| c == '=')
}

#[test]
fn test_find_block() {
    assert!(find_block(
        "
============================= xGPUs Indexed by PID ==============================
============================= GPUs Indexed by PID ==============================
PID 25774 is using 1 DRM device(s):
0
================================================================================
",
        "= GPUs Indexed by PID ="
    )
    .eq(&vec!["PID 25774 is using 1 DRM device(s):", "0"]))
}
