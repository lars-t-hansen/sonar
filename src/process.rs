// Run "ps" and return a vector of structures with all the information we need.

use crate::command::{self, CmdError};
use crate::jobs;
use crate::util;

#[derive(PartialEq)]
pub struct Process {
    pub pid: usize,
    pub user: String,
    pub cpu_pct: f64,
    pub mem_pct: f64,
    pub mem_size_kib: usize,
    pub command: String,
    pub ppid: usize,    // 0 if !jobs.need_process_tree()
    pub session: usize, // 0 if !jobs.need_process_tree()
}

pub fn get_process_information(jobs: &mut dyn jobs::JobManager) -> Result<Vec<Process>, CmdError> {
    let need_process_tree = jobs.need_process_tree();
    match command::safe_command(
        if need_process_tree {
            PS_COMMAND_COMPLETE
        } else {
            PS_COMMAND_FILTERED
        },
        TIMEOUT_SECONDS,
    ) {
        Ok(out) => Ok(parse_ps_output(&out, need_process_tree)),
        Err(e) => Err(e)
    }
}

const TIMEOUT_SECONDS: u64 = 2; // for `ps`

const PS_COMMAND_FILTERED: &str =
    "ps -e --no-header -o pid,user:22,pcpu,pmem,size,comm | grep -v ' 0.0  0.0 '";

const PS_COMMAND_COMPLETE: &str = "ps -e --no-header -o pid,user:22,pcpu,pmem,size,ppid,sess,comm";

fn parse_ps_output(raw_text: &str, complete_output: bool) -> Vec<Process> {
    raw_text
        .lines()
        .map(|line| {
            let (start_indices, parts) = util::chunks(line);
            Process {
                pid: parts[0].parse::<usize>().unwrap(),
                user: parts[1].to_string(),
                cpu_pct: parts[2].parse::<f64>().unwrap(),
                mem_pct: parts[3].parse::<f64>().unwrap(),
                mem_size_kib: parts[4].parse::<usize>().unwrap(),
                ppid: if complete_output {
                    parts[5].to_string().parse::<usize>().unwrap()
                } else {
                    0
                },
                session: if complete_output {
                    parts[6].to_string().parse::<usize>().unwrap()
                } else {
                    0
                },
                // this is done because command can have spaces
                command: line[start_indices[if complete_output { 7 } else { 5 }]..].to_string(),
            }
        })
        .collect::<Vec<Process>>()
}

#[cfg(test)]
pub fn parsed_test_output() -> Vec<Process> {
    let text = "   2022 bob                            10.0 20.0 553348 slack
  42178 bob                            10.0 15.0 353348 chromium
  42178 bob                            10.0 15.0  5536 chromium
  42189 alice                          10.0  5.0  5528 slack
  42191 bob                            10.0  5.0  5552 someapp
  42213 alice                          10.0  5.0 348904 some app
  42213 alice                          10.0  5.0 135364 some app";

    parse_ps_output(text)
}

#[test]
fn test_parse_ps_output() {
    macro_rules! proc(
	{ $a:expr, $b:expr, $c:expr, $d:expr, $e: expr, $f:expr } => {
	    Process { pid: $a,
		      user: $b.to_string(),
		      cpu_pct: $c,
		      mem_pct: $d,
		      mem_size_kib: $e,
		      command: $f.to_string()
	    }
	});

    assert!(parsed_test_output().into_iter().eq(vec![
        proc! {  2022, "bob",   10.0, 20.0, 553348, "slack" },
        proc! { 42178, "bob",   10.0, 15.0, 353348, "chromium" },
        proc! { 42178, "bob",   10.0, 15.0,   5536, "chromium" },
        proc! { 42189, "alice", 10.0,  5.0,   5528, "slack" },
        proc! { 42191, "bob",   10.0,  5.0,   5552, "someapp" },
        proc! { 42213, "alice", 10.0,  5.0, 348904, "some app" },
        proc! { 42213, "alice", 10.0,  5.0, 135364, "some app" }
    ]))
}

#[cfg(test)]
pub fn parsed_full_test_output() -> Vec<Process> {
    // Generated by PS_COMMAND_COMPLETE on lth's laptop, slightly edited to orphan #80199
    //"ps -e --no-header -o pid,user:22,pcpu,pmem,size,ppid,sess,comm"
    // pid user                pcpu pmen  size     ppid    sess command
    let text =
"      1 root                    0.0  0.0 21516       0       1 systemd
      2 root                    0.0  0.0     0       0       0 kthreadd
      3 root                    0.0  0.0     0       2       0 rcu_gp
      4 root                    0.0  0.0     0       2       0 rcu_par_gp
      5 root                    0.0  0.0     0       2       0 slub_flushwq
      6 root                    0.0  0.0     0       2       0 netns
      8 root                    0.0  0.0     0       2       0 kworker/0:0H-events_highpri
     10 root                    0.0  0.0     0       2       0 mm_percpu_wq
     11 root                    0.0  0.0     0       2       0 rcu_tasks_kthread
     12 root                    0.0  0.0     0       2       0 rcu_tasks_rude_kthread
     13 root                    0.0  0.0     0       2       0 rcu_tasks_trace_kthread
     14 root                    0.0  0.0     0       2       0 ksoftirqd/0
     15 root                    0.0  0.0     0       2       0 rcu_preempt
     16 root                    0.0  0.0     0       2       0 migration/0
     17 root                    0.0  0.0     0       2       0 idle_inject/0
     19 root                    0.0  0.0     0       2       0 cpuhp/0
     20 root                    0.0  0.0     0       2       0 cpuhp/1
     21 root                    0.0  0.0     0       2       0 idle_inject/1
     22 root                    0.0  0.0     0       2       0 migration/1
     23 root                    0.0  0.0     0       2       0 ksoftirqd/1
     25 root                    0.0  0.0     0       2       0 kworker/1:0H-events_highpri
     26 root                    0.0  0.0     0       2       0 cpuhp/2
     27 root                    0.0  0.0     0       2       0 idle_inject/2
     28 root                    0.0  0.0     0       2       0 migration/2
     29 root                    0.0  0.0     0       2       0 ksoftirqd/2
     31 root                    0.0  0.0     0       2       0 kworker/2:0H-events_highpri
     32 root                    0.0  0.0     0       2       0 cpuhp/3
     33 root                    0.0  0.0     0       2       0 idle_inject/3
     34 root                    0.0  0.0     0       2       0 migration/3
     35 root                    0.0  0.0     0       2       0 ksoftirqd/3
     37 root                    0.0  0.0     0       2       0 kworker/3:0H-events_highpri
     38 root                    0.0  0.0     0       2       0 cpuhp/4
     39 root                    0.0  0.0     0       2       0 idle_inject/4
     40 root                    0.0  0.0     0       2       0 migration/4
     41 root                    0.0  0.0     0       2       0 ksoftirqd/4
     43 root                    0.0  0.0     0       2       0 kworker/4:0H-kblockd
     44 root                    0.0  0.0     0       2       0 cpuhp/5
     45 root                    0.0  0.0     0       2       0 idle_inject/5
     46 root                    0.0  0.0     0       2       0 migration/5
     47 root                    0.0  0.0     0       2       0 ksoftirqd/5
     49 root                    0.0  0.0     0       2       0 kworker/5:0H-events_highpri
     50 root                    0.0  0.0     0       2       0 cpuhp/6
     51 root                    0.0  0.0     0       2       0 idle_inject/6
     52 root                    0.0  0.0     0       2       0 migration/6
     53 root                    0.0  0.0     0       2       0 ksoftirqd/6
     55 root                    0.0  0.0     0       2       0 kworker/6:0H-events_highpri
     56 root                    0.0  0.0     0       2       0 cpuhp/7
     57 root                    0.0  0.0     0       2       0 idle_inject/7
     58 root                    0.0  0.0     0       2       0 migration/7
     59 root                    0.0  0.0     0       2       0 ksoftirqd/7
     61 root                    0.0  0.0     0       2       0 kworker/7:0H-events_highpri
     62 root                    0.0  0.0     0       2       0 kdevtmpfs
     63 root                    0.0  0.0     0       2       0 inet_frag_wq
     64 root                    0.0  0.0     0       2       0 kauditd
     65 root                    0.0  0.0     0       2       0 khungtaskd
     67 root                    0.0  0.0     0       2       0 oom_reaper
     69 root                    0.0  0.0     0       2       0 writeback
     70 root                    0.0  0.0     0       2       0 kcompactd0
     71 root                    0.0  0.0     0       2       0 ksmd
     72 root                    0.0  0.0     0       2       0 khugepaged
     73 root                    0.0  0.0     0       2       0 kintegrityd
     74 root                    0.0  0.0     0       2       0 kblockd
     75 root                    0.0  0.0     0       2       0 blkcg_punt_bio
     78 root                    0.0  0.0     0       2       0 tpm_dev_wq
     79 root                    0.0  0.0     0       2       0 ata_sff
     81 root                    0.0  0.0     0       2       0 md
     82 root                    0.0  0.0     0       2       0 edac-poller
     83 root                    0.0  0.0     0       2       0 devfreq_wq
     84 root                    0.0  0.0     0       2       0 watchdogd
     85 root                    0.0  0.0     0       2       0 kworker/0:1H-acpi_thermal_pm
     86 root                    0.0  0.0     0       2       0 kswapd0
     87 root                    0.0  0.0     0       2       0 ecryptfs-kthread
     93 root                    0.0  0.0     0       2       0 kthrotld
     98 root                    0.0  0.0     0       2       0 irq/124-pciehp
     99 root                    0.0  0.0     0       2       0 irq/125-pciehp
    104 root                    0.0  0.0     0       2       0 acpi_thermal_pm
    105 root                    0.0  0.0     0       2       0 xenbus_probe
    107 root                    0.0  0.0     0       2       0 vfio-irqfd-clea
    108 root                    0.0  0.0     0       2       0 mld
    109 root                    0.0  0.0     0       2       0 kworker/5:1H-kblockd
    110 root                    0.0  0.0     0       2       0 ipv6_addrconf
    115 root                    0.0  0.0     0       2       0 kstrp
    121 root                    0.0  0.0     0       2       0 zswap-shrink
    170 root                    0.0  0.0     0       2       0 charger_manager
    208 root                    0.0  0.0     0       2       0 kworker/7:1H-events_highpri
    229 root                    0.0  0.0     0       2       0 kworker/3:1H-events_highpri
    231 root                    0.0  0.0     0       2       0 nvme-wq
    232 root                    0.0  0.0     0       2       0 nvme-reset-wq
    233 root                    0.0  0.0     0       2       0 nvme-delete-wq
    238 root                    0.0  0.0     0       2       0 irq/173-SYNA30B7:00
    239 root                    0.0  0.0     0       2       0 kworker/2:1H-events_highpri
    243 root                    0.0  0.0     0       2       0 irq/174-WACF4233:00
    267 root                    0.0  0.0     0       2       0 jbd2/nvme0n1p2-8
    268 root                    0.0  0.0     0       2       0 ext4-rsv-conver
    303 root                    0.0  0.0     0       2       0 kworker/6:1H-kblockd
    308 root                    0.0  0.3 18052       1     308 systemd-journal
    335 root                    0.0  0.0     0       2       0 kworker/4:1H-events_highpri
    336 root                    0.0  0.0     0       2       0 kworker/1:1H-events_highpri
    339 root                    0.0  0.0  2676       1     339 systemd-udevd
    469 root                    0.0  0.0     0       2       0 cfg80211
    485 root                    0.0  0.0     0       2       0 irq/175-iwlwifi:default_queue
    488 root                    0.0  0.0     0       2       0 irq/176-iwlwifi:queue_1
    489 root                    0.0  0.0     0       2       0 irq/177-iwlwifi:queue_2
    490 root                    0.0  0.0     0       2       0 irq/178-iwlwifi:queue_3
    491 root                    0.0  0.0     0       2       0 irq/179-iwlwifi:queue_4
    492 root                    0.0  0.0     0       2       0 irq/180-iwlwifi:queue_5
    493 root                    0.0  0.0     0       2       0 irq/181-iwlwifi:queue_6
    494 root                    0.0  0.0     0       2       0 irq/182-iwlwifi:queue_7
    496 root                    0.0  0.0     0       2       0 irq/183-iwlwifi:queue_8
    498 root                    0.0  0.0     0       2       0 irq/184-iwlwifi:exception
    512 systemd-oom             0.0  0.0   740       1     512 systemd-oomd
    513 systemd-resolve         0.0  0.0  5204       1     513 systemd-resolve
    514 systemd-timesync        0.0  0.0  8944       1     514 systemd-timesyn
    535 root                    0.0  0.0     0       2       0 cryptd
    581 root                    0.0  0.0 25828       1     581 accounts-daemon
    584 root                    0.0  0.0   360       1     584 acpid
    587 avahi                   0.0  0.0   636       1     587 avahi-daemon
    589 root                    0.0  0.0   440       1     589 cron
    590 messagebus              0.0  0.0  3512       1     590 dbus-daemon
    592 root                    0.0  0.1 28332       1     592 NetworkManager
    602 root                    0.0  0.0  8916       1     602 irqbalance
    616 root                    0.0  0.1 10896       1     616 networkd-dispat
    617 root                    0.0  0.0 28820       1     617 polkitd
    618 root                    0.0  0.0 25796       1     618 power-profiles-
    619 syslog                  0.0  0.0 18708       1     619 rsyslogd
    621 root                    0.0  0.2 263568      1     621 snapd
    626 root                    0.0  0.0 25828       1     626 switcheroo-cont
    643 root                    0.0  0.0 33780       1     643 systemd-logind
    654 root                    0.0  0.0 25984       1     654 thermald
    655 root                    0.0  0.0 43880       1     655 udisksd
    677 root                    0.0  0.0  2020       1     677 wpa_supplicant
    687 avahi                   0.0  0.0   448     587     587 avahi-daemon
    719 root                    0.0  0.0 34868       1     719 ModemManager
    722 root                    0.0  0.0 25764       1     722 boltd
    751 root                    0.0  0.1 18004       1     751 unattended-upgr
    757 root                    0.0  0.0 26100       1     757 gdm3
    761 root                    0.0  0.0 32580       1     761 iio-sensor-prox
    792 root                    0.0  0.0   584       1     792 bluetoothd
    799 root                    0.0  0.0     0       2       0 card0-crtc0
    800 root                    0.0  0.0     0       2       0 card0-crtc1
    801 root                    0.0  0.0     0       2       0 card0-crtc2
    802 root                    0.0  0.0     0       2       0 card0-crtc3
    960 root                    0.0  0.0     0       2       0 irq/207-AudioDSP
   1079 rtkit                   0.0  0.0 17076       1    1079 rtkit-daemon
   1088 root                    0.0  0.0 26144       1    1088 upowerd
   1352 root                    0.0  0.2 50776       1    1352 packagekitd
   1523 colord                  0.0  0.0 28708       1    1523 colord
   1618 kernoops                0.0  0.0   520       1    1618 kerneloops
   1622 kernoops                0.0  0.0   520       1    1622 kerneloops
   1789 root                    0.0  0.0 35428     757     757 gdm-session-wor
   1804 larstha                 0.0  0.0  2216       1    1804 systemd
   1805 larstha                 0.0  0.0 20556    1804    1804 (sd-pam)
   1811 larstha                 0.0  0.0 25636    1804    1811 pipewire
   1812 larstha                 0.0  0.0  9256    1804    1812 pipewire-media-
   1813 larstha                 0.1  0.1 72012    1804    1813 pulseaudio
   1823 larstha                 0.0  0.0  2624    1804    1823 dbus-daemon
   1825 larstha                 0.0  0.0 59244       1    1824 gnome-keyring-d
   1834 larstha                 0.0  0.0 25792    1804    1834 gvfsd
   1840 larstha                 0.0  0.0 44420    1804    1834 gvfsd-fuse
   1855 larstha                 0.0  0.0 60976    1804    1855 xdg-document-po
   1859 larstha                 0.0  0.0 25536    1804    1859 xdg-permission-
   1865 root                    0.0  0.0   356    1855    1865 fusermount3
   1884 larstha                 0.0  0.1 151232   1804    1884 tracker-miner-f
   1892 root                    0.0  0.0     0       2       0 krfcommd
   1894 larstha                 0.0  0.0 35316    1804    1894 gvfs-udisks2-vo
   1899 larstha                 0.0  0.0 25708    1804    1899 gvfs-mtp-volume
   1903 larstha                 0.0  0.0 25688    1804    1903 gvfs-goa-volume
   1907 larstha                 0.0  0.2 44544    1804    1823 goa-daemon
   1914 larstha                 0.0  0.0 34564    1804    1823 goa-identity-se
   1916 larstha                 0.0  0.0 33936    1804    1916 gvfs-afc-volume
   1925 larstha                 0.0  0.0 26124    1804    1925 gvfs-gphoto2-vo
   1938 larstha                 0.0  0.0 17216    1789    1938 gdm-wayland-ses
   1943 larstha                 0.0  0.0 17924    1938    1938 gnome-session-b
   1985 larstha                 0.0  0.0  8836    1804    1985 gnome-session-c
   1997 larstha                 0.0  0.1 52144    1804    1997 gnome-session-b
   2019 larstha                 0.6  2.2 375812   1804    2019 gnome-shell
   2020 larstha                 0.0  0.0 33988    1997    1997 at-spi-bus-laun
   2028 larstha                 0.0  0.0   788    2020    1997 dbus-daemon
   2136 larstha                 0.0  0.0 17372    1804    2136 gvfsd-metadata
   2144 larstha                 0.0  0.1 60144    1804    1823 gnome-shell-cal
   2150 larstha                 0.0  0.1 61688    1804    2150 evolution-sourc
   2163 larstha                 0.0  0.0 17460    1804    2163 dconf-service
   2168 larstha                 0.0  0.1 103436   1804    2168 evolution-calen
   2183 larstha                 0.0  0.1 77172    1804    2183 evolution-addre
   2198 larstha                 0.0  0.1 56024    1804    1823 gjs
   2200 larstha                 0.0  0.0 17364    1804    1997 at-spi2-registr
   2208 larstha                 0.0  0.0 34376    1834    1834 gvfsd-trash
   2222 larstha                 0.0  0.0   364    1804    2222 sh
   2223 larstha                 0.0  0.0 34020    1804    2223 gsd-a11y-settin
   2225 larstha                 0.0  0.0 38596    2222    2222 ibus-daemon
   2226 larstha                 0.0  0.1 63708    1804    2226 gsd-color
   2229 larstha                 0.0  0.0 34656    1804    2229 gsd-datetime
   2231 larstha                 0.0  0.0 34200    1804    2231 gsd-housekeepin
   2232 larstha                 0.0  0.1 45964    1804    2232 gsd-keyboard
   2233 larstha                 0.0  0.1 46408    1804    2233 gsd-media-keys
   2234 larstha                 0.0  0.1 47436    1804    2234 gsd-power
   2236 larstha                 0.0  0.0 26092    1804    2236 gsd-print-notif
   2238 larstha                 0.0  0.0 50668    1804    2238 gsd-rfkill
   2239 larstha                 0.0  0.0 25560    1804    2239 gsd-screensaver
   2240 larstha                 0.0  0.0 51732    1804    2240 gsd-sharing
   2241 larstha                 0.0  0.0 42500    1804    2241 gsd-smartcard
   2242 larstha                 0.0  0.0 34220    1804    2242 gsd-sound
   2243 larstha                 0.0  0.1 46256    1804    2243 gsd-wacom
   2303 larstha                 0.0  0.0 17372    2225    2222 ibus-memconf
   2305 larstha                 0.0  0.1 43832    2225    2222 ibus-extension-
   2308 larstha                 0.0  0.0 25756    1804    1823 ibus-portal
   2311 larstha                 0.0  0.3 76628    1997    1997 evolution-alarm
   2319 larstha                 0.0  0.0 26612    1997    1997 gsd-disk-utilit
   2375 larstha                 0.0  1.7 321276   1804    1997 snap-store
   2417 larstha                 0.0  0.0 17820    2225    2222 ibus-engine-sim
   2465 larstha                 0.0  0.0 34612    1804    2236 gsd-printer
   2520 larstha                 0.0  0.0 76956    1804    2520 xdg-desktop-por
   2530 larstha                 0.0  0.1 68100    1804    2530 xdg-desktop-por
   2555 larstha                 0.0  0.1 48012    1804    1823 gjs
   2573 larstha                 0.0  0.1 39892    1804    2573 xdg-desktop-por
   2636 root                    0.0  0.5 108880      1    2636 fwupd
   2656 larstha                 0.0  0.0  1280    1804    2656 snapd-desktop-i
   2734 larstha                 0.0  0.1 31484    2656    2656 snapd-desktop-i
   3325 larstha                 0.1  0.7 122884   2019    2019 Xwayland
   3344 larstha                 0.0  0.4 102844   1804    3344 gsd-xsettings
   3375 larstha                 0.0  0.1 23424    1804    3344 ibus-x11
   3884 larstha                 0.0  0.1 212236   1804    1823 snap
   5131 larstha                 0.0  0.1 48764    1997    1997 update-notifier
   7780 larstha                 0.0  0.0 26112    1834    1834 gvfsd-http
   9221 larstha                 0.0  0.4 73636    1804    9221 gnome-terminal-
   9239 larstha                 0.0  0.0  3636    9221    9239 bash
  11438 larstha                 0.0  0.8 236224   2019    2019 obsidian
  11495 larstha                 0.0  0.3  4920   11438    2019 obsidian
  11496 larstha                 0.0  0.2  4904   11438    2019 obsidian
  11526 larstha                 0.0  0.8 207856  11495    2019 obsidian
  11531 larstha                 0.0  0.4 63952   11438    2019 obsidian
  11542 larstha                 0.0  1.0 287796  11438    2019 obsidian
  11543 larstha                 0.0  1.2 337172  11438    2019 obsidian
  12887 larstha                 0.0  0.0  1076    1825    1824 ssh-agent
  74536 larstha                 0.0  0.0  3052    9221   74536 bash
  80195 larstha                 0.0  0.3 84612    1804    1823 gnome-calendar
  80199 larstha                 0.0  0.2 46812     200    1823 seahorse
  82329 larstha                 0.5  4.1 1090880  2019    2019 firefox
  82497 larstha                 0.0  0.2 13656   82329    2019 Socket Process
  82516 larstha                 0.0  0.6 82080   82329    2019 Privileged Cont
  82554 larstha                 0.0  1.6 358988  82329    2019 Isolated Web Co
  82558 larstha                 0.0  1.9 331480  82329    2019 Isolated Web Co
  82562 larstha                 0.0  2.7 541812  82329    2019 Isolated Web Co
  82572 larstha                 0.0  1.9 323628  82329    2019 Isolated Web Co
  82584 larstha                 0.0  0.6 62756   82329    2019 Isolated Web Co
  82605 larstha                 0.0  1.3 208208  82329    2019 Isolated Web Co
  82631 larstha                 0.0  0.9 112432  82329    2019 Isolated Web Co
  82652 larstha                 0.0  2.1 483464  82329    2019 Isolated Web Co
  82680 larstha                 0.0  2.0 333032  82329    2019 Isolated Web Co
  82732 larstha                 0.0  1.9 338896  82329    2019 Isolated Web Co
  83002 larstha                 0.0  1.0 261228  82329    2019 WebExtensions
  83286 larstha                 0.0  2.3 425108  82329    2019 Isolated Web Co
  83326 larstha                 0.0  1.1 160964  82329    2019 Isolated Web Co
  83332 larstha                 0.0  0.2 39804   82329    2019 RDD Process
  83340 larstha                 0.0  0.2 17728   82329    2019 Utility Process
  83618 larstha                 0.0  1.2 212360  82329    2019 Isolated Web Co
  83689 larstha                 0.0  1.0 136256  82329    2019 Isolated Web Co
  83925 larstha                 0.0  1.3 205144  82329    2019 Isolated Web Co
  84013 larstha                 0.0  1.0 141120  82329    2019 Isolated Web Co
  84177 larstha                 0.0  1.9 329400  82329    2019 Isolated Web Co
  96883 larstha                 0.0  1.0 174652  82329    2019 Isolated Web Co
  97718 larstha                 0.0  0.8 107784  82329    2019 Isolated Web Co
  99395 larstha                 0.0  0.7 78764   82329    2019 Isolated Web Co
  99587 larstha                 0.0  0.8 106744  82329    2019 Isolated Web Co
 103356 larstha                 0.0  0.7 77912   82329    2019 Isolated Web Co
 103359 larstha                 0.0  0.8 111172  82329    2019 Isolated Web Co
 103470 larstha                 0.0  0.7 99448   82329    2019 file:// Content
 104433 larstha                 0.0  3.5 669636  82329    2019 Isolated Web Co
 104953 larstha                 0.0  2.7 399200  82329    2019 Isolated Web Co
 116260 larstha                 0.0  0.8 103444  82329    2019 Isolated Web Co
 116296 larstha                 0.0  0.7 80048   82329    2019 Isolated Web Co
 116609 larstha                 0.0  0.7 99424   82329    2019 Isolated Web Co
 116645 larstha                 0.0  0.7 78512   82329    2019 Isolated Web Co
 116675 larstha                 0.0  1.1 150372  82329    2019 Isolated Web Co
 116997 larstha                 0.0  1.8 280516  82329    2019 Isolated Web Co
 119104 larstha                 0.0  1.1 191908  82329    2019 Isolated Web Co
 119151 larstha                 0.0  1.0 147144  82329    2019 Isolated Web Co
 128778 larstha                 0.1  0.4 78964    2019    2019 emacs
 132391 larstha                 0.0  0.8 101260  82329    2019 Isolated Web Co
 133097 larstha                 0.1  1.3 278532  82329    2019 Isolated Web Co
 134154 larstha                 0.0  0.6 64788   82329    2019 Isolated Web Co
 135609 larstha                 0.0  0.7 77260   82329    2019 Isolated Web Co
 136169 root                    0.0  0.0     0       2       0 kworker/u17:1-i915_flip
 140722 larstha                 0.0  0.8 96308   82329    2019 Isolated Web Co
 142642 root                    0.0  0.0     0       2       0 kworker/u17:0-i915_flip
 144346 root                    0.0  0.0     0       2       0 kworker/1:1-events
 144602 root                    0.0  0.0     0       2       0 kworker/u16:57-events_unbound
 144609 root                    0.0  0.0     0       2       0 kworker/u16:64-events_power_efficient
 144624 root                    0.0  0.0     0       2       0 irq/185-mei_me
 144736 root                    0.0  0.0  7960       1  144736 cupsd
 144754 root                    0.0  0.0 18104       1  144754 cups-browsed
 145490 larstha                 0.0  0.5 84372    2019    2019 gjs
 145716 root                    0.0  0.0     0       2       0 kworker/7:2-events
 146289 root                    0.0  0.0     0       2       0 kworker/u16:0-events_power_efficient
 146290 root                    0.1  0.0     0       2       0 kworker/6:1-events
 146342 root                    0.0  0.0     0       2       0 kworker/2:1-events
 146384 root                    0.0  0.0     0       2       0 kworker/5:0-events
 146735 root                    0.0  0.0     0       2       0 kworker/0:0-events
 146791 root                    0.0  0.0     0       2       0 kworker/1:2-events
 147017 root                    0.0  0.0     0       2       0 kworker/4:2-events
 147313 root                    0.0  0.0     0       2       0 kworker/3:2-events
 147413 root                    0.0  0.0     0       2       0 kworker/7:0-mm_percpu_wq
 147421 root                    0.0  0.0     0       2       0 kworker/6:2-inet_frag_wq
 147709 root                    0.0  0.0     0       2       0 kworker/2:2-events
 147914 root                    0.0  0.0     0       2       0 kworker/5:2-events
 147916 root                    0.0  0.0     0       2       0 kworker/4:0-events
 147954 root                    0.0  0.0     0       2       0 kworker/1:3-mm_percpu_wq
 148064 root                    0.0  0.0     0       2       0 kworker/3:0-events
 148065 root                    0.0  0.0     0       2       0 kworker/0:2-events
 148141 root                    0.0  0.0     0       2       0 kworker/7:1-events
 148142 root                    0.0  0.0     0       2       0 kworker/u17:2
 148173 root                    0.1  0.0     0       2       0 kworker/6:0-events
 148253 root                    0.0  0.0     0       2       0 kworker/2:0
 148259 larstha                 0.0  0.4 45648   82329    2019 Isolated Servic
 148284 root                    0.0  0.0     0       2       0 kworker/u16:1-events_power_efficient
 148286 root                    0.0  0.0     0       2       0 kworker/4:1-events_freezable
 148299 larstha                 0.0  0.4 38948   82329    2019 Web Content
 148301 larstha                 0.0  0.4 38952   82329    2019 Web Content
 148367 root                    0.1  0.0     0       2       0 kworker/3:1-events
 148371 root                    0.0  0.0     0       2       0 kworker/5:1-events
 148378 larstha                 0.4  0.3 38968   82329    2019 Web Content
 148406 larstha                 0.0  0.0  1100    9239    9239 ps
";
    parse_ps_output(text, true)
}
