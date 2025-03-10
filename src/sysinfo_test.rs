use crate::sysinfo;
use crate::mocksystem;
use crate::gpuapi;

use std::collections::HashMap;

// Test that the output is the expected output

#[test]
pub fn sysinfo_output_test() {
    let mut files = HashMap::new();
    files.insert("cpuinfo".to_string(), std::include_str!("testdata/cpuinfo.txt").to_string());
    files.insert(
        "meminfo".to_string(),
        "MemTotal:       16093776 kB".to_string(),
    );
    let system = mocksystem::MockSystem::new()
        .with_version("0.13.100")
        .with_timestamp("2025-02-11T08:47+01:00")
        .with_hostname("yes.no")
        .with_cluster("olivia.sigma2.no")
        .with_os("CP/M", "2.2")
        .with_files(files)
        .with_card(gpuapi::Card{
            bus_addr: "12:14:16".to_string(),
            device: gpuapi::GpuName{
                index: 0,
                uuid: "1234.5678".to_string(),
            },
            model: "Yoyodyne 1".to_string(),
            mem_size_kib: 1024*1024,
            power_limit_watt: 2000,
            max_power_limit_watt: 3000,
            max_ce_clock_mhz: 100000,
            ..Default::default()
        })
        .freeze();
    // CSV
    let mut output = Vec::new();
    sysinfo::show_system(&mut output, &system, true, false);
    let info = String::from_utf8_lossy(&output);
    let expect = r#"version=0.13.100,timestamp=2025-02-11T08:47+01:00,hostname=yes.no,"description=2x4 (hyperthreaded) Intel(R) Xeon(R) CPU E5-2637 v4 @ 3.50GHz, 15 GiB, 1x Yoyodyne 1 @ 1GiB",cpu_cores=16,mem_gb=15,gpu_cards=1,gpumem_gb=1,"gpu_info=""bus_addr=12:14:16,index=0,uuid=1234.5678,""""manufacturer=Yoyodyne, Inc."""",model=Yoyodyne 1,arch=,driver=,firmware=,mem_size_kib=1048576,power_limit_watt=2000,max_power_limit_watt=3000,min_power_limit_watt=0,max_ce_clock_mhz=100000,max_mem_clock_mhz=0"""
"#;
    assert!(info == expect);

    // Old JSON
    let mut output = Vec::new();
    sysinfo::show_system(&mut output, &system, false, false);
    let info = String::from_utf8_lossy(&output);
    let expect = r#"{"version":"0.13.100","timestamp":"2025-02-11T08:47+01:00","hostname":"yes.no","description":"2x4 (hyperthreaded) Intel(R) Xeon(R) CPU E5-2637 v4 @ 3.50GHz, 15 GiB, 1x Yoyodyne 1 @ 1GiB","cpu_cores":16,"mem_gb":15,"gpu_cards":1,"gpumem_gb":1,"gpu_info":[{"bus_addr":"12:14:16","index":0,"uuid":"1234.5678","manufacturer":"Yoyodyne, Inc.","model":"Yoyodyne 1","arch":"","driver":"","firmware":"","mem_size_kib":1048576,"power_limit_watt":2000,"max_power_limit_watt":3000,"min_power_limit_watt":0,"max_ce_clock_mhz":100000,"max_mem_clock_mhz":0}]}
"#;
    assert!(info == expect);

    // New JSON
    let mut output = Vec::new();
    sysinfo::show_system(&mut output, &system, false, true);
    let info = String::from_utf8_lossy(&output);
    let expect = r#"
{"meta":
{"producer":"sonar","version":"0.13.100"},
"data":
{
"type":"sysinfo",
"attributes":
{
"time":"2025-02-11T08:47+01:00",
"cluster":"olivia.sigma2.no",
"node":"yes.no",
"os-name":"CP/M",
"os-release":"2.2",
"cores":
[
{"index":0,"physical":0,"model":"Intel(R) Xeon(R) CPU E5-2637 v4 @ 3.50GHz"},
{"index":1,"physical":0,"model":"Intel(R) Xeon(R) CPU E5-2637 v4 @ 3.50GHz"},
{"index":2,"physical":1,"model":"Intel(R) Xeon(R) CPU E5-2637 v4 @ 3.50GHz"},
{"index":3,"physical":1,"model":"Intel(R) Xeon(R) CPU E5-2637 v4 @ 3.50GHz"},
{"index":4,"physical":2,"model":"Intel(R) Xeon(R) CPU E5-2637 v4 @ 3.50GHz"},
{"index":5,"physical":2,"model":"Intel(R) Xeon(R) CPU E5-2637 v4 @ 3.50GHz"},
{"index":6,"physical":3,"model":"Intel(R) Xeon(R) CPU E5-2637 v4 @ 3.50GHz"},
{"index":7,"physical":3,"model":"Intel(R) Xeon(R) CPU E5-2637 v4 @ 3.50GHz"},
{"index":8,"physical":4,"model":"Intel(R) Xeon(R) CPU E5-2637 v4 @ 3.50GHz"},
{"index":9,"physical":4,"model":"Intel(R) Xeon(R) CPU E5-2637 v4 @ 3.50GHz"},
{"index":10,"physical":5,"model":"Intel(R) Xeon(R) CPU E5-2637 v4 @ 3.50GHz"},
{"index":11,"physical":5,"model":"Intel(R) Xeon(R) CPU E5-2637 v4 @ 3.50GHz"},
{"index":12,"physical":6,"model":"Intel(R) Xeon(R) CPU E5-2637 v4 @ 3.50GHz"},
{"index":13,"physical":6,"model":"Intel(R) Xeon(R) CPU E5-2637 v4 @ 3.50GHz"},
{"index":14,"physical":7,"model":"Intel(R) Xeon(R) CPU E5-2637 v4 @ 3.50GHz"},
{"index":15,"physical":7,"model":"Intel(R) Xeon(R) CPU E5-2637 v4 @ 3.50GHz"}
],
"memory":16093776,
"description":"2x4 (hyperthreaded) Intel(R) Xeon(R) CPU E5-2637 v4 @ 3.50GHz, 15 GiB, 1x Yoyodyne 1 @ 1GiB",
"cards":[
{
"address":"12:14:16",
"index":0,
"uuid":"1234.5678",
"manufacturer":"Yoyodyne, Inc.",
"model":"Yoyodyne 1",
"architecture":"",
"driver":"",
"firmware":"",
"memory":1048576,
"power-limit":2000,
"max-power-limit":3000,
"min-power-limit":0,
"max-ce-clock":100000,
"max-mem-clock":0
}
]
}
}
}
"#;
    // println!("{}", info.replace('\n',""));
    // println!("{}", expect.replace('\n',""));
    assert!(info.replace('\n',"") == expect.replace('\n',""));
}
