// Could use bindgen but not important now

extern "C" {
    pub fn nvml_open() -> cty::c_int;
    pub fn nvml_close() -> cty::c_int;
    pub fn nvml_device_get_count(count: *mut cty::uint32_t) -> cty::c_int;
    pub fn nvml_device_get_architecture(device: cty::uint32_t, arch: *mut cty::uint32_t) -> cty::c_int;
    pub fn nvml_device_get_memory_info(device: cty::uint32_t, total: *mut cty::uint64_t, used: *mut cty::uint64_t, free: *mut cty::uint64_t) -> cty::c_int;
}

pub fn experiment() {
    println!("Experiment");
    unsafe {
        if nvml_open() != 0 {
            println!("nvml_open failed\n");
            return
        }

        let mut ndev: cty::uint32_t = 0;
        if nvml_device_get_count(&mut ndev) != 0 {
            println!("nvml_device_get_count returned 0\n");
            return
        }
        println!("devices: {ndev}");

        for i in 0..ndev {
            let mut arch: cty::uint32_t = 0;
            if nvml_device_get_architecture(i, &mut arch) != 0 {
                continue
            }
            println!("device_get_architecture {i} {arch}");

            let mut total: cty::uint64_t = 0;
            let mut used: cty::uint64_t = 0;
            let mut free: cty::uint64_t = 0;
            if nvml_device_get_memory_info(i, &mut total, &mut used, &mut free) != 0 {
                continue
            }
            println!("device_get_memory_info {i} {total} {used} {free}");
        }

        nvml_close();
    }
}
