use crate::gpu;
use std::ffi::CStr;

// Could use bindgen but not important now
extern "C" {
    pub fn nvml_open() -> cty::c_int;

    pub fn nvml_close() -> cty::c_int;

    pub fn nvml_device_get_count(count: *mut cty::uint32_t) -> cty::c_int;

    pub fn nvml_device_get_architecture(
        device: cty::uint32_t, arch: *mut cty::uint32_t) -> cty::c_int;

    pub fn nvml_device_get_memory_info(
        device: cty::uint32_t,
        total: *mut cty::uint64_t,
        used: *mut cty::uint64_t,
        free: *mut cty::uint64_t) -> cty::c_int;

    pub fn nvml_device_get_name(
        device: cty::uint32_t,
        buf: *mut cty::c_char,
        bufsiz: cty::size_t) -> cty::c_int;

    pub fn nvml_device_get_uuid(
        device: cty::uint32_t,
        buf: *mut cty::c_char,
        bufsiz: cty::size_t) -> cty::c_int;

    pub fn nvml_system_get_driver_version(
        buf: *mut cty::c_char, bufsiz: cty::size_t) -> cty::c_int;
}

pub fn get_cards() -> Option<Vec<gpu::Card>> {
    if unsafe { nvml_open() } != 0 {
        return None;
    }

    let mut num_devices: cty::uint32_t = 0;
    if unsafe { nvml_device_get_count(&mut num_devices) } != 0 {
        unsafe { nvml_close() };
        return None;
    }

    let driver = {
        const SIZE: usize = 128;
        let mut buffer = vec![0i8; SIZE];
        unsafe {
            if nvml_system_get_driver_version(buffer.as_mut_ptr(), SIZE) == 0 {
                CStr::from_ptr(buffer.as_ptr())
                    .to_str()
                    .expect("Will always be utf8")
            } else {
                "(unknown)"
            }.to_string()
        }
    };

    let mut result = vec![];
    for dev in 0..num_devices {
        let arch = {
            let mut arch: cty::uint32_t = 0;
            if unsafe { nvml_device_get_architecture(dev, &mut arch) } == 0 {
                match arch {
                    2 => "Kepler",
                    3 => "Maxwell",
                    4 => "Pascal",
                    5 => "Volta",
                    6 => "Turing",
                    7 => "Ampere",
                    8 => "Ada",
                    9 => "Hopper",
                    x => {
                        if x < 2 {
                            "(something old)"
                        } else {
                            "(something new)"
                        }
                    }
                }
            } else {
                "(unknown)"
            }.to_string()
        };

        let mem_size_kib = {
            let mut total: cty::uint64_t = 0;
            let mut used: cty::uint64_t = 0;
            let mut free: cty::uint64_t = 0;
            if unsafe { nvml_device_get_memory_info(dev, &mut total, &mut used, &mut free) } == 0 {
                (total / 1024) as i64
            } else {
                0
            }
        };

        let model = {
            const SIZE: usize = 128;
            let mut buffer = vec![0i8; SIZE];
            unsafe {
                if nvml_device_get_name(dev, buffer.as_mut_ptr(), SIZE) == 0 {
                    CStr::from_ptr(buffer.as_ptr())
                        .to_str()
                        .expect("Will always be utf8")
                } else {
                    "(unknown)"
                }.to_string()
            }
        };

        let uuid = {
            const SIZE: usize = 128;
            let mut buffer = vec![0i8; SIZE];
            unsafe {
                if nvml_device_get_uuid(dev, buffer.as_mut_ptr(), SIZE) == 0 {
                    CStr::from_ptr(buffer.as_ptr())
                        .to_str()
                        .expect("Will always be utf8")
                } else {
                    "(unknown)"
                }.to_string()
            }
        };

        result.push(gpu::Card{
            bus_addr: "".to_string(), // FIXME
            index: dev as i32,
            model,
            arch,
            driver: driver.clone(),
            firmware: "".to_string(), // FIXME
            uuid,
            mem_size_kib,
            power_limit_watt: 0, // FIXME
            max_power_limit_watt: 0, // FIXME
            min_power_limit_watt: 0, // FIXME
            max_ce_clock_mhz: 0, // FIXME
            max_mem_clock_mhz: 0, // FIXME
        })
    }

    unsafe { nvml_close() };
    Some(result)
}
