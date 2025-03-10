use crate::gpuapi;
use crate::output;
use crate::procfs;
use crate::systemapi;

use std::io;

pub fn show_system(writer: &mut dyn io::Write, system: &dyn systemapi::SystemAPI, csv: bool, new_json: bool) {
    let sysinfo = match compute_nodeinfo(system) {
        Ok(info) => {
            if !new_json {
                layout_sysinfo_oldfmt(system, info)
            } else {
                layout_sysinfo_newfmt(system, info)
            }
        }
        Err(e) => {
            if !new_json {
                layout_error_oldfmt(system, e)
            } else {
                layout_error_newfmt(system, e)
            }
        }
    };
    if csv {
        output::write_csv(writer, &output::Value::O(sysinfo));
    } else {
        output::write_json(writer, &output::Value::O(sysinfo));
    }
}

// New JSON format - json:api compatible, see spec.

fn layout_sysinfo_newfmt(system: &dyn systemapi::SystemAPI, node_info: NodeInfo) -> output::Object {
    let mut envelope = output::newfmt_envelope(system);
    let (mut data, mut attrs) = output::newfmt_data(system, "sysinfo");
    attrs.push_s("node", node_info.node.clone());
    attrs.push_s("os-name", system.get_os_name());
    attrs.push_s("os-release", system.get_os_release());
    let mut cores = output::Array::new();
    for k in &node_info.cores {
        let mut c = output::Object::new();
        c.push_i("index", k.logical_index);
        c.push_i("physical", k.physical_index);
        c.push_s("model", k.model.clone());
        cores.push_o(c);
    }
    attrs.push_a("cores", cores);
    attrs.push_i("memory", node_info.mem_kb);
    attrs.push_s("description", node_info.description.clone());
    // FIXME: topo-svg - comes from running lstopo I guess.
    let gpu_info = layout_card_info(&node_info, true);
    if gpu_info.len() > 0 {
        attrs.push_a("cards", gpu_info);
    }
    // FIXME: software - this is very TBD.
    data.push_o("attributes", attrs);
    envelope.push_o("data", data);
    envelope
}

fn layout_error_newfmt(system: &dyn systemapi::SystemAPI, error: String) -> output::Object {
    let mut envelope = output::newfmt_envelope(system);
    envelope.push_a("errors", output::newfmt_one_error(system, error));
    envelope
}

// Old JSON/CSV format - this is flattish (to accomodate CSV) and has some idiosyncratic field names.

fn layout_sysinfo_oldfmt(system: &dyn systemapi::SystemAPI, node_info: NodeInfo) -> output::Object {
    let gpu_info = layout_card_info(&node_info, false);
    let mut sysinfo = output::Object::new();
    sysinfo.push_s("version", system.get_version());
    sysinfo.push_s("timestamp", system.get_timestamp());
    sysinfo.push_s("hostname", system.get_hostname());
    sysinfo.push_s("description", node_info.description);
    sysinfo.push_i("cpu_cores", node_info.cpu_cores);
    sysinfo.push_i("mem_gb", node_info.mem_kb / (1024 * 1024));
    if node_info.gpu_cards != 0 {
        sysinfo.push_i("gpu_cards", node_info.gpu_cards);
        if node_info.gpumem_kb != 0 {
            sysinfo.push_i("gpumem_gb", node_info.gpumem_kb / (1024 * 1024));
        }
        if gpu_info.len() > 0 {
            sysinfo.push_a("gpu_info", gpu_info);
        }
    }
    sysinfo
}

fn layout_error_oldfmt(system: &dyn systemapi::SystemAPI, error: String) -> output::Object {
    let mut sysinfo = output::Object::new();
    sysinfo.push_s("version", system.get_version());
    sysinfo.push_s("timestamp", system.get_timestamp());
    sysinfo.push_s("hostname", system.get_hostname());
    sysinfo.push_s("error", error);
    sysinfo
}

// Old and new formats layout the cards the same way but use different field names in a number of
// cases.

fn layout_card_info(node_info: &NodeInfo, new_json: bool) -> output::Array {
    let mut gpu_info = output::Array::new();
    for c in &node_info.cards {
        let gpuapi::Card {
            device,
            bus_addr,
            model,
            arch,
            driver,
            firmware,
            mem_size_kib,
            power_limit_watt,
            max_power_limit_watt,
            min_power_limit_watt,
            max_ce_clock_mhz,
            max_mem_clock_mhz,
        } = c;
        let mut gpu = output::Object::new();
        gpu.push_s(if new_json { "address" } else { "bus_addr" },
                   bus_addr.to_string());
        gpu.push_i("index", device.index as i64);
        gpu.push_s("uuid", device.uuid.to_string());
        gpu.push_s("manufacturer", node_info.card_manufacturer.clone());
        gpu.push_s("model", model.to_string());
        gpu.push_s(if new_json { "architecture" } else { "arch" },
                   arch.to_string());
        gpu.push_s("driver", driver.to_string());
        gpu.push_s("firmware", firmware.to_string());
        gpu.push_i(if new_json { "memory" } else { "mem_size_kib" },
                   *mem_size_kib);
        gpu.push_i(if new_json { "power-limit" } else { "power_limit_watt" },
                   *power_limit_watt as i64);
        gpu.push_i(if new_json { "max-power-limit" } else { "max_power_limit_watt" },
                   *max_power_limit_watt as i64);
        gpu.push_i(if new_json { "min-power-limit" } else { "min_power_limit_watt" },
                   *min_power_limit_watt as i64);
        gpu.push_i(if new_json { "max-ce-clock" } else { "max_ce_clock_mhz" },
                   *max_ce_clock_mhz as i64);
        gpu.push_i(if new_json { "max-mem-clock" } else { "max_mem_clock_mhz" },
                   *max_mem_clock_mhz as i64);
        gpu_info.push_o(gpu);
    }
    gpu_info
}

const GIB: usize = 1024 * 1024 * 1024;

struct CoreInfo {
    logical_index: i64,
    physical_index: i64,
    model: String,
}

struct NodeInfo {
    node: String,
    description: String,
    cpu_cores: i64,
    cores: Vec<CoreInfo>,
    mem_kb: i64,
    card_manufacturer: String,
    gpu_cards: i64,
    gpumem_kb: i64,
    cards: Vec<gpuapi::Card>,
}

fn compute_nodeinfo(system: &dyn systemapi::SystemAPI) -> Result<NodeInfo, String> {
    let fs = system.get_procfs();
    let gpus = system.get_gpus();
    let procfs::CpuInfo { sockets, cores_per_socket, threads_per_core, cores: sys_cores } =
        procfs::get_cpu_info(fs)?;
    let mut cores = Vec::new();
    let model_name = sys_cores[0].model_name.clone(); // expedient
    for c in sys_cores {
        cores.push(CoreInfo {
            logical_index: c.logical_index as i64,
            physical_index: c.physical_index as i64,
            model: c.model_name,
        })
    }
    let mem_kb = procfs::get_memtotal_kib(fs)?;
    let mem_gb = (mem_kb as f64 / (1024.0 * 1024.0)).round() as i64;
    let (mut cards, manufacturer) = match gpus.probe() {
        Some(device) => (
            device.get_card_configuration().unwrap_or_default(),
            device.get_manufacturer(),
        ),
        None => (vec![], "UNKNOWN".to_string()),
    };
    let ht = if threads_per_core > 1 {
        " (hyperthreaded)"
    } else {
        ""
    };

    let (gpu_desc, gpu_cards, gpumem_kb) = if !cards.is_empty() {
        // Sort cards
        cards.sort_by(|a: &gpuapi::Card, b: &gpuapi::Card| {
            if a.model == b.model {
                a.mem_size_kib.cmp(&b.mem_size_kib)
            } else {
                a.model.cmp(&b.model)
            }
        });

        // Merge equal cards
        let mut i = 0;
        let mut gpu_desc = "".to_string();
        while i < cards.len() {
            let first = i;
            while i < cards.len()
                && cards[i].model == cards[first].model
                && cards[i].mem_size_kib == cards[first].mem_size_kib
            {
                i += 1;
            }
            let memsize = if cards[first].mem_size_kib > 0 {
                ((cards[first].mem_size_kib as f64 * 1024.0 / GIB as f64).round() as usize)
                    .to_string()
            } else {
                "unknown ".to_string()
            };
            gpu_desc += &format!(", {}x {} @ {}GiB", (i - first), cards[first].model, memsize);
        }

        // Compute aggregate data
        let gpu_cards = cards.len() as i32;
        let mut total_mem_kb = 0i64;
        for c in &cards {
            total_mem_kb += c.mem_size_kib;
        }
        (gpu_desc, gpu_cards, total_mem_kb)
    } else {
        ("".to_string(), 0, 0)
    };
    Ok(NodeInfo {
        node: system.get_hostname(),
        description: format!("{sockets}x{cores_per_socket}{ht} {model_name}, {mem_gb} GiB{gpu_desc}"),
        cpu_cores: (sockets * cores_per_socket * threads_per_core) as i64,
        cores,
        mem_kb: mem_kb as i64,
        card_manufacturer: manufacturer,
        gpu_cards: gpu_cards as i64,
        gpumem_kb,
        cards,
    })
}
