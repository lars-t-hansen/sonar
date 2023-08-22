/// Collect CPU process information without GPU information.

use crate::command::{self, CmdError};
use crate::procfs;
use crate::util;

#[derive(PartialEq)]
pub struct Process {
    pub pid: usize,
    pub uid: usize,
    pub user: String,
    pub cpu_pct: f64,
    pub mem_pct: f64,
    pub cputime_sec: usize,
    pub mem_size_kib: usize,
    pub command: String,
    pub ppid: usize,
    pub session: usize,
}

/// Obtain process information and return a vector of structures with all the information we need.
/// In the returned vector, pids uniquely tag the records.
///
/// This will attempt to get the values from /proc/PID/{stat,comm,statm} first, and if that
/// fails, it will run ps.

pub fn get_process_information() -> Result<Vec<Process>, CmdError> {
    if let Some(result) = procfs::get_process_information() {
        Ok(result)
    } else {
        match command::safe_command(PS_COMMAND, TIMEOUT_SECONDS) {
            Ok(out) => Ok(parse_ps_output(&out)),
            Err(e) => Err(e),
        }
    }
}

const TIMEOUT_SECONDS: u64 = 2; // for `ps`

// `--cumulative` and `bsdtime` are to make sure that the cpu time accounted to exited child
// processes (the cutime and cstime fields of /proc/pid/status) is used and printed.  Note
// `cputimes` is unaffected by `--cumulative`.
//
// The format of `bsdtime` is `m...m:ss` in minutes and seconds.

const PS_COMMAND: &str =
    "ps -e --no-header --cumulative -o pid,uid,user:22,pcpu,pmem,bsdtime,size,ppid,sess,comm";

fn parse_ps_output(raw_text: &str) -> Vec<Process> {
    raw_text
        .lines()
        .map(|line| {
            let (start_indices, parts) = util::chunks(line);
            Process {
                pid: parts[0].parse::<usize>().unwrap(),
                uid: parts[1].parse::<usize>().unwrap(),
                user: parts[2].to_string(),
                cpu_pct: parts[3].parse::<f64>().unwrap(),
                mem_pct: parts[4].parse::<f64>().unwrap(),
                cputime_sec: parse_bsdtime(parts[5]),
                mem_size_kib: parts[6].parse::<usize>().unwrap(),
                ppid: parts[7].to_string().parse::<usize>().unwrap(),
                session: parts[8].to_string().parse::<usize>().unwrap(),
                // this is done because command can have spaces
                command: line[start_indices[9]..].to_string(),
            }
        })
        .collect::<Vec<Process>>()
}

fn parse_bsdtime<'a>(s: &'a str) -> usize {
    let ss = s.split(':').collect::<Vec<&'a str>>();
    if ss.len() != 2 {
        0
    } else {
        ss[0].parse::<usize>().unwrap() * 60 + ss[1].parse::<usize>().unwrap()
    }
}

#[cfg(test)]
pub fn parsed_test_output() -> Vec<Process> {
    let text = "   2022 1001 bob                            10.0 20.0 1:28 553348 1234 0 slack
  42178 1001 bob                            10.0 15.0 1:29 353348 1235 1 chromium
  42178 1001 bob                            10.0 15.0 1:30 5536  1236 2 chromium
  42189 1002 alice                          10.0  5.0 1:31 5528  1237 3 slack
  42191 1001 bob                            10.0  5.0 1:32 5552  1238 4 someapp
  42213 1002 alice                          10.0  5.0 1:33 348904 1239 5 some app
  42213 1002 alice                          10.0  5.0 1:34 135364 1240 6 some app";

    parse_ps_output(text)
}

#[test]
fn test_parse_ps_output() {
    macro_rules! proc(
        { $a:expr, $b:expr, $c:expr, $d:expr, $e: expr, $f:expr, $g:expr, $h:expr, $i:expr, $j:expr } => {
            Process { pid: $a,
                      uid: $b,
                      user: $c.to_string(),
                      cpu_pct: $d,
                      mem_pct: $e,
                      cputime_sec: $f,
                      ppid: $g,
                      mem_size_kib: $h,
                      session: $i,
                      command: $j.to_string(),
            }
        });

    assert!(parsed_test_output().into_iter().eq(vec![
        proc! {  2022, 1001, "bob",   10.0, 20.0, 60+28, 1234, 553348, 0, "slack" },
        proc! { 42178, 1001, "bob",   10.0, 15.0, 60+29, 1235, 353348, 1, "chromium" },
        proc! { 42178, 1001, "bob",   10.0, 15.0, 60+30, 1236,   5536, 2, "chromium" },
        proc! { 42189, 1002, "alice", 10.0,  5.0, 60+31, 1237,  5528, 3, "slack" },
        proc! { 42191, 1001, "bob",   10.0,  5.0, 60+32, 1238,  5552, 4, "someapp" },
        proc! { 42213, 1002, "alice", 10.0,  5.0, 60+33, 1239, 348904, 5, "some app" },
        proc! { 42213, 1002, "alice", 10.0,  5.0, 60+34, 1240, 135364, 6, "some app" }
    ]))
}

#[cfg(test)]
pub fn parsed_full_test_output() -> Vec<Process> {
    // Generated by PS_COMMAND_COMPLETE on lth's laptop, slightly edited to orphan #80199
    //"ps -e --no-header -o pid,user:22,pcpu,pmem,size,ppid,sess,comm"
    // Subsequently added synthetic cputimes number
    // pid user                pcpu pmem  cputimes size     ppid    sess command
    let text =
"      1 0 root                    0.0  0.0 1:28 21516       0       1 systemd
      2 0 root                    0.0  0.0     1:28 0       0       0 kthreadd
      3 0 root                    0.0  0.0     1:28 0       2       0 rcu_gp
      4 0 root                    0.0  0.0     1:28 0       2       0 rcu_par_gp
      5 0 root                    0.0  0.0     1:28 0       2       0 slub_flushwq
      6 0 root                    0.0  0.0     1:28 0       2       0 netns
      8 0 root                    0.0  0.0     1:28 0       2       0 kworker/0:0H-events_highpri
     10 0 root                    0.0  0.0     1:28 0       2       0 mm_percpu_wq
     11 0 root                    0.0  0.0     1:28 0       2       0 rcu_tasks_kthread
     12 0 root                    0.0  0.0     1:28 0       2       0 rcu_tasks_rude_kthread
     13 0 root                    0.0  0.0     1:28 0       2       0 rcu_tasks_trace_kthread
     14 0 root                    0.0  0.0     1:28 0       2       0 ksoftirqd/0
     15 0 root                    0.0  0.0     1:28 0       2       0 rcu_preempt
     16 0 root                    0.0  0.0     1:28 0       2       0 migration/0
     17 0 root                    0.0  0.0     1:28 0       2       0 idle_inject/0
     19 0 root                    0.0  0.0     1:28 0       2       0 cpuhp/0
     20 0 root                    0.0  0.0     1:28 0       2       0 cpuhp/1
     21 0 root                    0.0  0.0     1:28 0       2       0 idle_inject/1
     22 0 root                    0.0  0.0     1:28 0       2       0 migration/1
     23 0 root                    0.0  0.0     1:28 0       2       0 ksoftirqd/1
     25 0 root                    0.0  0.0     1:28 0       2       0 kworker/1:0H-events_highpri
     26 0 root                    0.0  0.0     1:28 0       2       0 cpuhp/2
     27 0 root                    0.0  0.0     1:28 0       2       0 idle_inject/2
     28 0 root                    0.0  0.0     1:28 0       2       0 migration/2
     29 0 root                    0.0  0.0     1:28 0       2       0 ksoftirqd/2
     31 0 root                    0.0  0.0     1:28 0       2       0 kworker/2:0H-events_highpri
     32 0 root                    0.0  0.0     1:28 0       2       0 cpuhp/3
     33 0 root                    0.0  0.0     1:28 0       2       0 idle_inject/3
     34 0 root                    0.0  0.0     1:28 0       2       0 migration/3
     35 0 root                    0.0  0.0     1:28 0       2       0 ksoftirqd/3
     37 0 root                    0.0  0.0     1:28 0       2       0 kworker/3:0H-events_highpri
     38 0 root                    0.0  0.0     1:28 0       2       0 cpuhp/4
     39 0 root                    0.0  0.0     1:28 0       2       0 idle_inject/4
     40 0 root                    0.0  0.0     1:28 0       2       0 migration/4
     41 0 root                    0.0  0.0     1:28 0       2       0 ksoftirqd/4
     43 0 root                    0.0  0.0     1:28 0       2       0 kworker/4:0H-kblockd
     44 0 root                    0.0  0.0     1:28 0       2       0 cpuhp/5
     45 0 root                    0.0  0.0     1:28 0       2       0 idle_inject/5
     46 0 root                    0.0  0.0     1:28 0       2       0 migration/5
     47 0 root                    0.0  0.0     1:28 0       2       0 ksoftirqd/5
     49 0 root                    0.0  0.0     1:28 0       2       0 kworker/5:0H-events_highpri
     50 0 root                    0.0  0.0     1:28 0       2       0 cpuhp/6
     51 0 root                    0.0  0.0     1:28 0       2       0 idle_inject/6
     52 0 root                    0.0  0.0     1:28 0       2       0 migration/6
     53 0 root                    0.0  0.0     1:28 0       2       0 ksoftirqd/6
     55 0 root                    0.0  0.0     1:28 0       2       0 kworker/6:0H-events_highpri
     56 0 root                    0.0  0.0     1:28 0       2       0 cpuhp/7
     57 0 root                    0.0  0.0     1:28 0       2       0 idle_inject/7
     58 0 root                    0.0  0.0     1:28 0       2       0 migration/7
     59 0 root                    0.0  0.0     1:28 0       2       0 ksoftirqd/7
     61 0 root                    0.0  0.0     1:28 0       2       0 kworker/7:0H-events_highpri
     62 0 root                    0.0  0.0     1:28 0       2       0 kdevtmpfs
     63 0 root                    0.0  0.0     1:28 0       2       0 inet_frag_wq
     64 0 root                    0.0  0.0     1:28 0       2       0 kauditd
     65 0 root                    0.0  0.0     1:28 0       2       0 khungtaskd
     67 0 root                    0.0  0.0     1:28 0       2       0 oom_reaper
     69 0 root                    0.0  0.0     1:28 0       2       0 writeback
     70 0 root                    0.0  0.0     1:28 0       2       0 kcompactd0
     71 0 root                    0.0  0.0     1:28 0       2       0 ksmd
     72 0 root                    0.0  0.0     1:28 0       2       0 khugepaged
     73 0 root                    0.0  0.0     1:28 0       2       0 kintegrityd
     74 0 root                    0.0  0.0     1:28 0       2       0 kblockd
     75 0 root                    0.0  0.0     1:28 0       2       0 blkcg_punt_bio
     78 0 root                    0.0  0.0     1:28 0       2       0 tpm_dev_wq
     79 0 root                    0.0  0.0     1:28 0       2       0 ata_sff
     81 0 root                    0.0  0.0     1:28 0       2       0 md
     82 0 root                    0.0  0.0     1:28 0       2       0 edac-poller
     83 0 root                    0.0  0.0     1:28 0       2       0 devfreq_wq
     84 0 root                    0.0  0.0     1:28 0       2       0 watchdogd
     85 0 root                    0.0  0.0     1:28 0       2       0 kworker/0:1H-acpi_thermal_pm
     86 0 root                    0.0  0.0     1:28 0       2       0 kswapd0
     87 0 root                    0.0  0.0     1:28 0       2       0 ecryptfs-kthread
     93 0 root                    0.0  0.0     1:28 0       2       0 kthrotld
     98 0 root                    0.0  0.0     1:28 0       2       0 irq/124-pciehp
     99 0 root                    0.0  0.0     1:28 0       2       0 irq/125-pciehp
    104 0 root                    0.0  0.0     1:28 0       2       0 acpi_thermal_pm
    105 0 root                    0.0  0.0     1:28 0       2       0 xenbus_probe
    107 0 root                    0.0  0.0     1:28 0       2       0 vfio-irqfd-clea
    108 0 root                    0.0  0.0     1:28 0       2       0 mld
    109 0 root                    0.0  0.0     1:28 0       2       0 kworker/5:1H-kblockd
    110 0 root                    0.0  0.0     1:28 0       2       0 ipv6_addrconf
    115 0 root                    0.0  0.0     1:28 0       2       0 kstrp
    121 0 root                    0.0  0.0     1:28 0       2       0 zswap-shrink
    170 0 root                    0.0  0.0     1:28 0       2       0 charger_manager
    208 0 root                    0.0  0.0     1:28 0       2       0 kworker/7:1H-events_highpri
    229 0 root                    0.0  0.0     1:28 0       2       0 kworker/3:1H-events_highpri
    231 0 root                    0.0  0.0     1:28 0       2       0 nvme-wq
    232 0 root                    0.0  0.0     1:28 0       2       0 nvme-reset-wq
    233 0 root                    0.0  0.0     1:28 0       2       0 nvme-delete-wq
    238 0 root                    0.0  0.0     1:28 0       2       0 irq/173-SYNA30B7:00
    239 0 root                    0.0  0.0     1:28 0       2       0 kworker/2:1H-events_highpri
    243 0 root                    0.0  0.0     1:28 0       2       0 irq/174-WACF4233:00
    267 0 root                    0.0  0.0     1:28 0       2       0 jbd2/nvme0n1p2-8
    268 0 root                    0.0  0.0     1:28 0       2       0 ext4-rsv-conver
    303 0 root                    0.0  0.0     1:28 0       2       0 kworker/6:1H-kblockd
    308 0 root                    0.0  0.3 1:28 18052       1     308 systemd-journal
    335 0 root                    0.0  0.0     1:28 0       2       0 kworker/4:1H-events_highpri
    336 0 root                    0.0  0.0     1:28 0       2       0 kworker/1:1H-events_highpri
    339 0 root                    0.0  0.0  1:28 2676       1     339 systemd-udevd
    469 0 root                    0.0  0.0     1:28 0       2       0 cfg80211
    485 0 root                    0.0  0.0     1:28 0       2       0 irq/175-iwlwifi:default_queue
    488 0 root                    0.0  0.0     1:28 0       2       0 irq/176-iwlwifi:queue_1
    489 0 root                    0.0  0.0     1:28 0       2       0 irq/177-iwlwifi:queue_2
    490 0 root                    0.0  0.0     1:28 0       2       0 irq/178-iwlwifi:queue_3
    491 0 root                    0.0  0.0     1:28 0       2       0 irq/179-iwlwifi:queue_4
    492 0 root                    0.0  0.0     1:28 0       2       0 irq/180-iwlwifi:queue_5
    493 0 root                    0.0  0.0     1:28 0       2       0 irq/181-iwlwifi:queue_6
    494 0 root                    0.0  0.0     1:28 0       2       0 irq/182-iwlwifi:queue_7
    496 0 root                    0.0  0.0     1:28 0       2       0 irq/183-iwlwifi:queue_8
    498 0 root                    0.0  0.0     1:28 0       2       0 irq/184-iwlwifi:exception
    512 1 systemd-oom             0.0  0.0 1:33   740       1     512 systemd-oomd
    513 2 systemd-resolve         0.0  0.0 1:33  5204       1     513 systemd-resolve
    514 3 systemd-timesync        0.0  0.0 1:33  8944       1     514 systemd-timesyn
    535 0 root                    0.0  0.0 1:33     0       2       0 cryptd
    581 0 root                    0.0  0.0 1:33 25828       1     581 accounts-daemon
    584 0 root                    0.0  0.0 1:33   360       1     584 acpid
    587 4 avahi                   0.0  0.0 1:33   636       1     587 avahi-daemon
    589 0 root                    0.0  0.0 1:33   440       1     589 cron
    590 5 messagebus              0.0  0.0 1:33  3512       1     590 dbus-daemon
    592 0 root                    0.0  0.1 1:33 28332       1     592 NetworkManager
    602 0 root                    0.0  0.0 1:33  8916       1     602 irqbalance
    616 0 root                    0.0  0.1 1:33 10896       1     616 networkd-dispat
    617 0 root                    0.0  0.0 1:33 28820       1     617 polkitd
    618 0 root                    0.0  0.0 1:33 25796       1     618 power-profiles-
    619 6 syslog                  0.0  0.0 1:33 18708       1     619 rsyslogd
    621 0 root                    0.0  0.2 1:33 263568      1     621 snapd
    626 0 root                    0.0  0.0 1:33 25828       1     626 switcheroo-cont
    643 0 root                    0.0  0.0 1:33 33780       1     643 systemd-logind
    654 0 root                    0.0  0.0 1:33 25984       1     654 thermald
    655 0 root                    0.0  0.0 1:33 43880       1     655 udisksd
    677 0 root                    0.0  0.0 1:33  2020       1     677 wpa_supplicant
    687 4 avahi                   0.0  0.0 1:33   448     587     587 avahi-daemon
    719 0 root                    0.0  0.0 1:33 34868       1     719 ModemManager
    722 0 root                    0.0  0.0 1:33 25764       1     722 boltd
    751 0 root                    0.0  0.1 1:33 18004       1     751 unattended-upgr
    757 0 root                    0.0  0.0 1:33 26100       1     757 gdm3
    761 0 root                    0.0  0.0 1:33 32580       1     761 iio-sensor-prox
    792 0 root                    0.0  0.0 1:33   584       1     792 bluetoothd
    799 0 root                    0.0  0.0 1:33     0       2       0 card0-crtc0
    800 0 root                    0.0  0.0 1:33     0       2       0 card0-crtc1
    801 0 root                    0.0  0.0 1:33     0       2       0 card0-crtc2
    802 0 root                    0.0  0.0 1:33     0       2       0 card0-crtc3
    960 0 root                    0.0  0.0 1:33     0       2       0 irq/207-AudioDSP
   1079 7 rtkit                   0.0  0.0 1:33 17076       1    1079 rtkit-daemon
   1088 0 root                    0.0  0.0 1:33 26144       1    1088 upowerd
   1352 0 root                    0.0  0.2 1:33 50776       1    1352 packagekitd
   1523 8 colord                  0.0  0.0 1:33 28708       1    1523 colord
   1618 9 kernoops                0.0  0.0 1:33   520       1    1618 kerneloops
   1622 9 kernoops                0.0  0.0 1:33   520       1    1622 kerneloops
   1789 0 root                    0.0  0.0 1:33 35428     757     757 gdm-session-wor
   1804 1001 larstha                 0.0  0.0 1:33  2216       1    1804 systemd
   1805 1001 larstha                 0.0  0.0 1:33 20556    1804    1804 (sd-pam)
   1811 1001 larstha                 0.0  0.0 1:33 25636    1804    1811 pipewire
   1812 1001 larstha                 0.0  0.0 1:33  9256    1804    1812 pipewire-media-
   1813 1001 larstha                 0.1  0.1 1:33 72012    1804    1813 pulseaudio
   1823 1001 larstha                 0.0  0.0 1:33  2624    1804    1823 dbus-daemon
   1825 1001 larstha                 0.0  0.0 1:33 59244       1    1824 gnome-keyring-d
   1834 1001 larstha                 0.0  0.0 1:33 25792    1804    1834 gvfsd
   1840 1001 larstha                 0.0  0.0 1:33 44420    1804    1834 gvfsd-fuse
   1855 1001 larstha                 0.0  0.0 1:33 60976    1804    1855 xdg-document-po
   1859 1001 larstha                 0.0  0.0 1:33 25536    1804    1859 xdg-permission-
   1865 0 root                    0.0  0.0 1:33   356    1855    1865 fusermount3
   1884 1001 larstha                 0.0  0.1 1:33 151232   1804    1884 tracker-miner-f
   1892 0 root                    0.0  0.0 1:33     0       2       0 krfcommd
   1894 1001 larstha                 0.0  0.0 1:33 35316    1804    1894 gvfs-udisks2-vo
   1899 1001 larstha                 0.0  0.0 1:33 25708    1804    1899 gvfs-mtp-volume
   1903 1001 larstha                 0.0  0.0 1:33 25688    1804    1903 gvfs-goa-volume
   1907 1001 larstha                 0.0  0.2 1:33 44544    1804    1823 goa-daemon
   1914 1001 larstha                 0.0  0.0 1:33 34564    1804    1823 goa-identity-se
   1916 1001 larstha                 0.0  0.0 1:33 33936    1804    1916 gvfs-afc-volume
   1925 1001 larstha                 0.0  0.0 1:33 26124    1804    1925 gvfs-gphoto2-vo
   1938 1001 larstha                 0.0  0.0 1:33 17216    1789    1938 gdm-wayland-ses
   1943 1001 larstha                 0.0  0.0 1:33 17924    1938    1938 gnome-session-b
   1985 1001 larstha                 0.0  0.0 1:33  8836    1804    1985 gnome-session-c
   1997 1001 larstha                 0.0  0.1 1:33 52144    1804    1997 gnome-session-b
   2019 1001 larstha                 0.6  2.2 1:33 375812   1804    2019 gnome-shell
   2020 1001 larstha                 0.0  0.0 1:33 33988    1997    1997 at-spi-bus-laun
   2028 1001 larstha                 0.0  0.0 1:33   788    2020    1997 dbus-daemon
   2136 1001 larstha                 0.0  0.0 1:33 17372    1804    2136 gvfsd-metadata
   2144 1001 larstha                 0.0  0.1 1:33 60144    1804    1823 gnome-shell-cal
   2150 1001 larstha                 0.0  0.1 1:33 61688    1804    2150 evolution-sourc
   2163 1001 larstha                 0.0  0.0 1:33 17460    1804    2163 dconf-service
   2168 1001 larstha                 0.0  0.1 1:33 103436   1804    2168 evolution-calen
   2183 1001 larstha                 0.0  0.1 1:33 77172    1804    2183 evolution-addre
   2198 1001 larstha                 0.0  0.1 1:33 56024    1804    1823 gjs
   2200 1001 larstha                 0.0  0.0 1:33 17364    1804    1997 at-spi2-registr
   2208 1001 larstha                 0.0  0.0 1:33 34376    1834    1834 gvfsd-trash
   2222 1001 larstha                 0.0  0.0 1:33   364    1804    2222 sh
   2223 1001 larstha                 0.0  0.0 1:33 34020    1804    2223 gsd-a11y-settin
   2225 1001 larstha                 0.0  0.0 1:33 38596    2222    2222 ibus-daemon
   2226 1001 larstha                 0.0  0.1 1:33 63708    1804    2226 gsd-color
   2229 1001 larstha                 0.0  0.0 1:33 34656    1804    2229 gsd-datetime
   2231 1001 larstha                 0.0  0.0 1:33 34200    1804    2231 gsd-housekeepin
   2232 1001 larstha                 0.0  0.1 1:33 45964    1804    2232 gsd-keyboard
   2233 1001 larstha                 0.0  0.1 1:33 46408    1804    2233 gsd-media-keys
   2234 1001 larstha                 0.0  0.1 1:33 47436    1804    2234 gsd-power
   2236 1001 larstha                 0.0  0.0 1:33 26092    1804    2236 gsd-print-notif
   2238 1001 larstha                 0.0  0.0 1:33 50668    1804    2238 gsd-rfkill
   2239 1001 larstha                 0.0  0.0 1:33 25560    1804    2239 gsd-screensaver
   2240 1001 larstha                 0.0  0.0 1:33 51732    1804    2240 gsd-sharing
   2241 1001 larstha                 0.0  0.0 1:33 42500    1804    2241 gsd-smartcard
   2242 1001 larstha                 0.0  0.0 1:33 34220    1804    2242 gsd-sound
   2243 1001 larstha                 0.0  0.1 1:33 46256    1804    2243 gsd-wacom
   2303 1001 larstha                 0.0  0.0 1:33 17372    2225    2222 ibus-memconf
   2305 1001 larstha                 0.0  0.1 1:33 43832    2225    2222 ibus-extension-
   2308 1001 larstha                 0.0  0.0 1:33 25756    1804    1823 ibus-portal
   2311 1001 larstha                 0.0  0.3 1:33 76628    1997    1997 evolution-alarm
   2319 1001 larstha                 0.0  0.0 1:33 26612    1997    1997 gsd-disk-utilit
   2375 1001 larstha                 0.0  1.7 1:33 321276   1804    1997 snap-store
   2417 1001 larstha                 0.0  0.0 1:33 17820    2225    2222 ibus-engine-sim
   2465 1001 larstha                 0.0  0.0 1:33 34612    1804    2236 gsd-printer
   2520 1001 larstha                 0.0  0.0 1:33 76956    1804    2520 xdg-desktop-por
   2530 1001 larstha                 0.0  0.1 1:33 68100    1804    2530 xdg-desktop-por
   2555 1001 larstha                 0.0  0.1 1:33 48012    1804    1823 gjs
   2573 1001 larstha                 0.0  0.1 1:33 39892    1804    2573 xdg-desktop-por
   2636 0 root                    0.0  0.5 1:33 108880      1    2636 fwupd
   2656 1001 larstha                 0.0  0.0 1:33  1280    1804    2656 snapd-desktop-i
   2734 1001 larstha                 0.0  0.1 1:33 31484    2656    2656 snapd-desktop-i
   3325 1001 larstha                 0.1  0.7 1:33 122884   2019    2019 Xwayland
   3344 1001 larstha                 0.0  0.4 1:33 102844   1804    3344 gsd-xsettings
   3375 1001 larstha                 0.0  0.1 1:33 23424    1804    3344 ibus-x11
   3884 1001 larstha                 0.0  0.1 1:33 212236   1804    1823 snap
   5131 1001 larstha                 0.0  0.1 1:33 48764    1997    1997 update-notifier
   7780 1001 larstha                 0.0  0.0 1:33 26112    1834    1834 gvfsd-http
   9221 1001 larstha                 0.0  0.4 1:33 73636    1804    9221 gnome-terminal-
   9239 1001 larstha                 0.0  0.0 1:33  3636    9221    9239 bash
  11438 1001 larstha                 0.0  0.8 1:33 236224   2019    2019 obsidian
  11495 1001 larstha                 0.0  0.3 1:33  4920   11438    2019 obsidian
  11496 1001 larstha                 0.0  0.2 1:33  4904   11438    2019 obsidian
  11526 1001 larstha                 0.0  0.8 1:33 207856  11495    2019 obsidian
  11531 1001 larstha                 0.0  0.4 1:33 63952   11438    2019 obsidian
  11542 1001 larstha                 0.0  1.0 1:33 287796  11438    2019 obsidian
  11543 1001 larstha                 0.0  1.2 1:33 337172  11438    2019 obsidian
  12887 1001 larstha                 0.0  0.0 1:33  1076    1825    1824 ssh-agent
  74536 1001 larstha                 0.0  0.0 1:33  3052    9221   74536 bash
  80195 1001 larstha                 0.0  0.3 1:33 84612    1804    1823 gnome-calendar
  80199 1001 larstha                 0.0  0.2 1:33 46812     200    1823 seahorse
  82329 1001 larstha                 0.5  4.1 1:33 1090880  2019    2019 firefox
  82497 1001 larstha                 0.0  0.2 1:33 13656   82329    2019 Socket Process
  82516 1001 larstha                 0.0  0.6 1:33 82080   82329    2019 Privileged Cont
  82554 1001 larstha                 0.0  1.6 1:33 358988  82329    2019 Isolated Web Co
  82558 1001 larstha                 0.0  1.9 1:33 331480  82329    2019 Isolated Web Co
  82562 1001 larstha                 0.0  2.7 1:33 541812  82329    2019 Isolated Web Co
  82572 1001 larstha                 0.0  1.9 1:33 323628  82329    2019 Isolated Web Co
  82584 1001 larstha                 0.0  0.6 1:33 62756   82329    2019 Isolated Web Co
  82605 1001 larstha                 0.0  1.3 1:33 208208  82329    2019 Isolated Web Co
  82631 1001 larstha                 0.0  0.9 1:33 112432  82329    2019 Isolated Web Co
  82652 1001 larstha                 0.0  2.1 1:33 483464  82329    2019 Isolated Web Co
  82680 1001 larstha                 0.0  2.0 1:33 333032  82329    2019 Isolated Web Co
  82732 1001 larstha                 0.0  1.9 1:33 338896  82329    2019 Isolated Web Co
  83002 1001 larstha                 0.0  1.0 1:33 261228  82329    2019 WebExtensions
  83286 1001 larstha                 0.0  2.3 1:33 425108  82329    2019 Isolated Web Co
  83326 1001 larstha                 0.0  1.1 1:33 160964  82329    2019 Isolated Web Co
  83332 1001 larstha                 0.0  0.2 1:33 39804   82329    2019 RDD Process
  83340 1001 larstha                 0.0  0.2 1:33 17728   82329    2019 Utility Process
  83618 1001 larstha                 0.0  1.2 1:33 212360  82329    2019 Isolated Web Co
  83689 1001 larstha                 0.0  1.0 1:33 136256  82329    2019 Isolated Web Co
  83925 1001 larstha                 0.0  1.3 1:33 205144  82329    2019 Isolated Web Co
  84013 1001 larstha                 0.0  1.0 1:33 141120  82329    2019 Isolated Web Co
  84177 1001 larstha                 0.0  1.9 1:33 329400  82329    2019 Isolated Web Co
  96883 1001 larstha                 0.0  1.0 1:33 174652  82329    2019 Isolated Web Co
  97718 1001 larstha                 0.0  0.8 1:33 107784  82329    2019 Isolated Web Co
  99395 1001 larstha                 0.0  0.7 1:33 78764   82329    2019 Isolated Web Co
  99587 1001 larstha                 0.0  0.8 1:33 106744  82329    2019 Isolated Web Co
 103356 1001 larstha                 0.0  0.7 1:33 77912   82329    2019 Isolated Web Co
 103359 1001 larstha                 0.0  0.8 1:33 111172  82329    2019 Isolated Web Co
 103470 1001 larstha                 0.0  0.7 1:33 99448   82329    2019 file:// Content
 104433 1001 larstha                 0.0  3.5 1:33 669636  82329    2019 Isolated Web Co
 104953 1001 larstha                 0.0  2.7 1:33 399200  82329    2019 Isolated Web Co
 116260 1001 larstha                 0.0  0.8 1:33 103444  82329    2019 Isolated Web Co
 116296 1001 larstha                 0.0  0.7 1:33 80048   82329    2019 Isolated Web Co
 116609 1001 larstha                 0.0  0.7 1:33 99424   82329    2019 Isolated Web Co
 116645 1001 larstha                 0.0  0.7 1:33 78512   82329    2019 Isolated Web Co
 116675 1001 larstha                 0.0  1.1 1:33 150372  82329    2019 Isolated Web Co
 116997 1001 larstha                 0.0  1.8 1:33 280516  82329    2019 Isolated Web Co
 119104 1001 larstha                 0.0  1.1 1:33 191908  82329    2019 Isolated Web Co
 119151 1001 larstha                 0.0  1.0 1:33 147144  82329    2019 Isolated Web Co
 128778 1001 larstha                 0.1  0.4 1:33 78964    2019    2019 emacs
 132391 1001 larstha                 0.0  0.8 1:33 101260  82329    2019 Isolated Web Co
 133097 1001 larstha                 0.1  1.3 1:33 278532  82329    2019 Isolated Web Co
 134154 1001 larstha                 0.0  0.6 1:33 64788   82329    2019 Isolated Web Co
 135609 1001 larstha                 0.0  0.7 1:33 77260   82329    2019 Isolated Web Co
 136169 0 root                    0.0  0.0 1:33     0       2       0 kworker/u17:1-i915_flip
 140722 1001 larstha                 0.0  0.8 1:33 96308   82329    2019 Isolated Web Co
 142642 0 root                    0.0  0.0 1:33     0       2       0 kworker/u17:0-i915_flip
 144346 0 root                    0.0  0.0 1:33     0       2       0 kworker/1:1-events
 144602 0 root                    0.0  0.0 1:33     0       2       0 kworker/u16:57-events_unbound
 144609 0 root                    0.0  0.0 1:33     0       2       0 kworker/u16:64-events_power_efficient
 144624 0 root                    0.0  0.0 1:33     0       2       0 irq/185-mei_me
 144736 0 root                    0.0  0.0 1:33  7960       1  144736 cupsd
 144754 0 root                    0.0  0.0 1:33 18104       1  144754 cups-browsed
 145490 1001 larstha                 0.0  0.5 1:33 84372    2019    2019 gjs
 145716 0 root                    0.0  0.0 1:33     0       2       0 kworker/7:2-events
 146289 0 root                    0.0  0.0 1:33     0       2       0 kworker/u16:0-events_power_efficient
 146290 0 root                    0.1  0.0 1:33     0       2       0 kworker/6:1-events
 146342 0 root                    0.0  0.0 1:33     0       2       0 kworker/2:1-events
 146384 0 root                    0.0  0.0 1:33     0       2       0 kworker/5:0-events
 146735 0 root                    0.0  0.0 1:33     0       2       0 kworker/0:0-events
 146791 0 root                    0.0  0.0 1:33     0       2       0 kworker/1:2-events
 147017 0 root                    0.0  0.0 1:33     0       2       0 kworker/4:2-events
 147313 0 root                    0.0  0.0 1:33     0       2       0 kworker/3:2-events
 147413 0 root                    0.0  0.0 1:33     0       2       0 kworker/7:0-mm_percpu_wq
 147421 0 root                    0.0  0.0 1:33     0       2       0 kworker/6:2-inet_frag_wq
 147709 0 root                    0.0  0.0 1:33     0       2       0 kworker/2:2-events
 147914 0 root                    0.0  0.0 1:33     0       2       0 kworker/5:2-events
 147916 0 root                    0.0  0.0 1:33     0       2       0 kworker/4:0-events
 147954 0 root                    0.0  0.0 1:33     0       2       0 kworker/1:3-mm_percpu_wq
 148064 0 root                    0.0  0.0 1:33     0       2       0 kworker/3:0-events
 148065 0 root                    0.0  0.0 1:33     0       2       0 kworker/0:2-events
 148141 0 root                    0.0  0.0 1:33     0       2       0 kworker/7:1-events
 148142 0 root                    0.0  0.0 1:33     0       2       0 kworker/u17:2
 148173 0 root                    0.1  0.0 1:33     0       2       0 kworker/6:0-events
 148253 0 root                    0.0  0.0 1:33     0       2       0 kworker/2:0
 148259 1001 larstha                 0.0  0.4 1:33 45648   82329    2019 Isolated Servic
 148284 0 root                    0.0  0.0 1:33     0       2       0 kworker/u16:1-events_power_efficient
 148286 0 root                    0.0  0.0 1:33     0       2       0 kworker/4:1-events_freezable
 148299 1001 larstha                 0.0  0.4 1:33 38948   82329    2019 Web Content
 148301 1001 larstha                 0.0  0.4 1:33 38952   82329    2019 Web Content
 148367 0 root                    0.1  0.0 1:33     0       2       0 kworker/3:1-events
 148371 0 root                    0.0  0.0 1:33     0       2       0 kworker/5:1-events
 148378 1001 larstha                 0.4  0.3 1:33 38968   82329    2019 Web Content
 148406 1001 larstha                 0.0  0.0 1:33  1100    9239    9239 ps
";
    parse_ps_output(text)
}
