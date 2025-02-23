// TODO:
//  - tls
//  - authentication, authorization
//  - interrupts
//  - misc error handling
//  - misc cleanup

// In the "daemon mode", Sonar stays memory-resident and pushes data to a network sink.  In this
// mode, the only command line parameter is the name of a config file.
//
// The daemon is a multi-threaded system that performs system sampling, communicates with a Kafka
// broker, and handles interrupts and lock files.  See later.
//
//
// CONFIG FILE.
//
// The config file is an ini-type file.  Blank lines and lines starting with '#' are ignored.  Each
// section has a [section-name] header on a line by itself.  Within the sections, there are
// name=value pairs where names are simple identifiers matching /[a-zA-Z_][-a-zA-Z_0-9]*/ and values
// may be quoted with ', ", or `; these quotes are stripped.  Blanks before and after names and
// values are stripped.
//
// bool values are true or false.  Duration values express a time value using the syntax __h, __m,
// or __s, denoting hour, minute, or second values (uppercase HMS also allowed).  Values must be
// nonzero. For cadences, second values must divide a minute evently and be < 60, minute values must
// divide an hour evenly and < 60, and hour values must divide a day evenly or be a positive
// multiple of 24.
//
// The config file has a [global] section that controls general operation; a section for the
// transport type chosen, eg [kafka]; and a section each for the sonar operations, controlling their
// cadence and operation in the same way as normal command line switches.  For the Sonar operations,
// the cadence setting is required for the operation to be run, the command will be run at a time
// that is zero mod the cadence.
//
// [global] section:
//
//   cluster = <canonical cluster name>
//   role = node | master
//   lockdir = <string>                              # default none
//
//   The cluster name is required, eg fox.educloud.no.
//
//   The role determines how this daemon responds to control messages from a remote controller.  It
//   must be defined.  Only the string values listed are accepted.  A `node` typically provides
//   sample and sysinfo data only, a `master` often only slurm data.
//
//   If there is a lockdir then a lockfile is acquired when the daemon runs for the daemon's
//   lifetime, though if the daemon is reloaded by remote command the lock is relinquished
//   temporarily (and the restarted config file may name a different lockdir).
//
// [kafka] section (preliminary):
//
//   remote-host = <hostname and port>
//   poll-interval = <duration value>                # default 5m
//
//   The remote-host is required.  For Kafka it's usually host:port, eg localhost:9092 for a local
//   broker on the standard port.
//
//   TODO: more settings coming, for authentication and authorization
//
// [ps] section aka [sample] section:
//
//   cadence = <duration value>
//   exclude-system-jobs = <bool>                    # default true
//   load = <bool>                                   # default true
//   batchless = <bool>                              # default false
//   exclude-users = <comma-separated strings>       # default []
//   exclude-commands = <comma-separated strings>    # default []
//
// [sysinfo] section:
//
//   cadence = <duration value>
//   on-startup = <bool>                             # default true
//
//   If on-startup is true then a sysinfo operation will be executed every time the daemon is
//   started.
//
// [slurm] section:
//
//   cadence = <duration value>
//   window = <duration value>                       # default 2*cadence
//
//   The window is the sacct time window used for looking for data.
//
// [debug] section:
//
//   dump = bool                                     # default false
//   verbose = bool                                  # default false
//
// Data messages: These are sent under topics <cluster>.<data-type> where cluster is as configured
// in the [global] section and data-type is `sample`, `sysinfo`, and `slurm`.  The payload is always
// JSON.
//
// Control messages: These are sent under topics <cluster>.<role> where cluster is as configured
// in the [global] section and role is `node` or `master`.  These will have key and value as follows:
//
//   Key     Value      Meaning
//   ------- ---------- -------------------------------------------
//   exit    (none)     Terminate sonar immediately
//   dump    <boolean>  Enable or disable data dump (for debugging)
//
// Example compute-node file:
//
//  [global]
//  cluster = mlx.hpc.uio.no
//  role = node
//
//  [kafka]
//  remote-host = naic-monitor.uio.no:12345
//
//  [sample]
//  cadence = 5m
//  batchless = true
//
//  [sysinfo]
//  cadence = 24h
//
//
// THREADS AND I/O
//
// The main thread of the daemon listens on a channel from which it reads events: alarms (for work
// to do), interrupts (from the keyboard), and control messages (from the Kafka broker).
//
// The Kafka thread handles interactions with the Kafka broker: it sends outgoing messages and
// reacts to incoming traffic.  Outgoing messages may be batched, the connection may go up and down,
// and so on.  Incoming traffic is forwarded to the main thread.  Outgoing traffic is received on a
// channel from the main thread.
//
// Interrupt handling is TBD but for practical purposes the interrupt system will look like a thread
// that receives keyboard interrupts and places them in the daemon's channel as events.

// At the moment we're going to try to make this work with the kafka-rust library
// (https://crates.io/crates/kafka), being 100% rust and somewhat lightweight.  The Kafka ties
// aren't too heavyweight anyway (I hope).

// Kafka issues:
// - local storage needs on node for offset store (.with_offset_storage looks scary)
// - Consumer is blocking unless with_fetch_max_wait_time in which case we'll busy-wait on poll,
//   not a happy case
//
// Probably also some kind of integration with the system object?

// Test notes with standard Kafka server, see https://kafka.apache.org/quickstart.
//
// In the first shell:
//
//   bin/zookeeper-server-start.sh config/zookeeper.properties
//
// In a second shell:
//
//   bin/kafka-server-start.sh config/server.properties
//
// Topics need to be added with kafka-topics a la this:
//
//   bin/kafka-topics.sh --create --topic mlx.hpc.uio.no.slurm --bootstrap-server localhost:9092
//
// The topics:
//
//   mlx.hpc.uio.no.sample
//   mlx.hpc.uio.no.sysinfo
//   mlx.hpc.uio.no.slurm
//   mlx.hpc.uio.no.node   // the control topic
//
// Run this in a shell to listen for sysinfo messages:
//
//   bin/kafka-console-consumer.sh --topic 'mlx.hpc.uio.no.sysinfo' --bootstrap-server localhost:9092
//
// Run this in a shell to listen for sample messages:
//
//   bin/kafka-console-consumer.sh --topic 'mlx.hpc.uio.no.sample' --bootstrap-server localhost:9092
//
// To send messages to sonar:
//
//   bin/kafka-console-producer.sh --bootstrap-server localhost:9092 --topic mlx.hpc.uio.no.node --property pars.key=true
//
// and then use TAB to separate key and value on each line.  A good test is `dump true` and `dump
// false`, but `exit` should work (without a value).

use crate::batchless;
use crate::realsystem;
use crate::ps;
use crate::sysinfo;
use crate::slurmjobs;
use crate::slurm;
use crate::time::{unix_now,unix_time_components};

use std::io::BufRead;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use kafka::producer;
use kafka::consumer;

struct GlobalIni {
    cluster: String,
    role: String,
    lockdir: Option<String>,
}

struct KafkaIni {
    remote_host: String,
    poll_interval: Dur,
}

struct DebugIni {
    dump: bool,
    verbose: bool,
}

struct SampleIni {
    cadence: Option<Dur>,
    exclude_system_jobs: bool,
    load: bool,
    batchless: bool,
    exclude_commands: Vec<String>,
    exclude_users: Vec<String>,
}

struct SysinfoIni {
    on_startup: bool,
    cadence: Option<Dur>,
}

struct SlurmIni {
    cadence: Option<Dur>,
    window: Option<Dur>,
}

struct Ini {
    global: GlobalIni,
    kafka: KafkaIni,
    debug: DebugIni,
    sample: SampleIni,
    sysinfo: SysinfoIni,
    slurm: SlurmIni,
}

#[derive(Clone,Copy)]
enum ControlOp {
    Exit,
    DumpOn,
    DumpOff,
}

#[derive(Clone,Copy)]
enum Operation {
    Sample,
    Sysinfo,
    Slurm,
    // Signals of various kinds
    Interrupt,
    // Control messages
    Control(ControlOp),
}

#[derive(Clone,Copy,PartialEq,Eq)]
enum Dur {
    Hours(u64),
    Minutes(u64),
    Seconds(u64),
}

impl Dur {
    fn to_seconds(&self) -> u64 {
        match self {
            Dur::Hours(n) => *n*60*60,
            Dur::Minutes(n) => *n*60,
            Dur::Seconds(n) => *n,
        }
    }
}

pub fn daemon_mode(config_file: &str, mut system: realsystem::RealSystemBuilder) -> Result<(), String> {
    let ini = parse_config(config_file)?;

    if ini.sample.cadence.is_some() {
        system = if ini.sample.batchless {
            system.with_jobmanager(Box::new(batchless::BatchlessJobManager::new()))
        } else {
            system.with_jobmanager(Box::new(slurm::SlurmJobManager {}))
        };
    }

    let system = system.freeze();

    let ps_opts = ps::PsOptions {
        rollup: false,
        always_print_something: true,
        min_cpu_percent: None,
        min_mem_percent: None,
        min_cpu_time: None,
        exclude_system_jobs: ini.sample.exclude_system_jobs,
        load: ini.sample.load,
        exclude_users: str_vec(&ini.sample.exclude_users),
        exclude_commands: str_vec(&ini.sample.exclude_commands),
        lockdir: ini.global.lockdir.clone(),
        json: true,
    };

    if ini.global.lockdir.is_some() {
        // TODO: Acquire lockdir here
        todo!();
    }

    // For communicating with the daemon thread.
    let (event_sender, event_receiver) = mpsc::channel();

    // For communicating with the Kafka thread.
    let (kafka_sender, kafka_receiver) = mpsc::channel();

    // If sysinfo runs on startup then post a message to ourselves.
    if ini.sysinfo.cadence.is_some() && ini.sysinfo.on_startup {
        event_sender.send(Operation::Sysinfo).unwrap();
    }

    // Alarms for daemon operations - each alarm gets its own thread, a little wasteful but OK for
    // the time being.  These will post the given events at the given cadence.
    if let Some(c) = ini.sysinfo.cadence {
        let sender = event_sender.clone();
        thread::spawn(move || { repeated_event(sender, Operation::Sysinfo, c); });
    }
    if let Some(c) = ini.sample.cadence {
        let sender = event_sender.clone();
        thread::spawn(move || { repeated_event(sender, Operation::Sample, c); });
    }
    if let Some(c) = ini.slurm.cadence {
        let sender = event_sender.clone();
        thread::spawn(move || { repeated_event(sender, Operation::Slurm, c); });
    }

    // Kafka daemon handlers - the producer and consumer run on separate threads, as that is what
    // the library prefers, everything is sync.  The daemon thread posts outgoing messages to the
    // producer thread (kafka_sender) and the kafka consumer thread posts incoming messages to the
    // daemon thread.
    {
        let host = ini.kafka.remote_host.clone();
        thread::spawn(move || { kafka_producer(kafka_receiver, host); });
    }

    {
        let host = ini.kafka.remote_host.clone();
        let control_topic = ini.global.cluster.clone() + "." + &ini.global.role;
        let sender = event_sender.clone();
        let poll_interval = ini.kafka.poll_interval;
        thread::spawn(move || { kafka_consumer(sender, host, control_topic, poll_interval); });
    }

    // Interrupt handlers - the logic here is that the interrupts will be handled in some low-level
    // manner and result in a message being posted to the daemon thread.
    thread::spawn(move || { interrupt_listener(event_sender); });

    let mut dump = ini.debug.dump;
    'messageloop:
    loop {
        let mut output = Vec::new();

        let topic : &'static str;
        match event_receiver.recv() {
            Err(e) => {
                return Err(format!("EXITING.  Event queue receive failed: {e}"));
            }
            Ok(op) => {
                system.update_time();
                match op {
                    Operation::Sample => {
                        if ini.debug.verbose {
                            println!("Sample");
                        }
                        ps::create_snapshot(&mut output, &system, &ps_opts);
                        topic = "sample";
                    }
                    Operation::Sysinfo => {
                        if ini.debug.verbose {
                            println!("Sysinfo");
                        }
                        sysinfo::show_system(&mut output, &system, false);
                        topic = "sysinfo";
                    }
                    Operation::Slurm => {
                        if ini.debug.verbose {
                            println!("Slurm");
                        }
                        let w = if let Some(c) = ini.slurm.window { Some(c.to_seconds() as u32) } else { None };
                        slurmjobs::show_slurm_jobs(&mut output, &w, &None, &system, true);
                        topic = "slurm";
                    }
                    Operation::Interrupt => {
                        if ini.debug.verbose {
                            println!("Interrupt");
                        }
                        // TODO: If exiting, relinquish lockdir
                        // TODO: Close kafka producer & consumer
                        todo!();
                        //continue 'messageloop;
                    }
                    Operation::Control(op) => {
                        if ini.debug.verbose {
                            println!("Control");
                        }
                        match op {
                            ControlOp::Exit => {
                                // TODO: Relinquish lockdir
                                // TODO: Close kafka producer & consumer
                                // TODO: What about interrupt thread?
                                // TODO: What about repeater threads?
                                // TODO: Return from function
                                todo!();
                            }
                            /*
                            ControlOp::Reload => {
                                // TODO: Relinquish lockdir
                                // TODO: Close kafka producer & consumer
                                // TODO: What about interrupt thread?
                                // TODO: What about repeater threads?
                                // TODO: Goto top of function, reloading the config file
                                todo!();
                            }
                            */
                            ControlOp::DumpOn => {
                                dump = true;
                            }
                            ControlOp::DumpOff => {
                                dump = false;
                            }
                        }
                        continue 'messageloop;
                    }
                }
            }
        }

        let s = String::from_utf8_lossy(&output).to_string();
        let topic = ini.global.cluster.clone() + "." + topic;

        if dump {
            println!("DUMP\nTOPIC: {}\n{}", topic, s);
        }

        kafka_sender.send((topic, s)).unwrap();
    }
}

fn str_vec<'a>(xs: &'a [String]) -> Vec<&'a str> {
    xs.iter().map(|x| x.as_str()).collect::<Vec<&'a str>>()
}

///////////////////////////////////////////////////////////////////////////////////////////////////
//
// Alarms and cadences.

fn repeated_event(sender: mpsc::Sender<Operation>, op: Operation, cadence: Dur) {
    let mut next = time_at_next_cadence_point(unix_now(), cadence);
    loop {
        let now = unix_now();
        let delay = next as i64 - now as i64;
        if delay > 0 {
            thread::sleep(std::time::Duration::from_secs(delay as u64));
        }
        sender.send(op).unwrap();
        next = now + cadence.to_seconds();
    }
}

// Round up `now` to the next multiple of `cadence`.  For example, if `cadence` is 5m then the value
// returned represents the unix time at the next 5 minute mark; if `cadence` is 24h then the value
// is the time at next midnight.  It's OK for this to be expensive (for now).  This can validly
// return `now`.
//
// The many restrictions on cadences ensure that this rounding is well-defined and leads to
// well-defined sample points across all nodes (that have sensibly synchronized clocks and are in
// compatible time zones).
//
// Multi-day boundaries are a little tricky but we can use the next midnight s.t.  the number of
// days evenly divides the day number.

fn time_at_next_cadence_point(now: u64, cadence: Dur) -> u64 {
    let (_,_,day,hour,minute,second) = unix_time_components(now);
    now + match cadence {
        Dur::Seconds(s) => {
            s - second % s
        }
        Dur::Minutes(m) => {
            60*(m - minute % m) - second
        }
        Dur::Hours(h) if h <= 24 => {
            60*(60*(h - hour % h) - minute) - second
        }
        Dur::Hours(h) => {
            let d = h/24;
            60*(60*(24*(d - day % d) - hour) - minute) - second
        }
    }
}

#[test]
pub fn test_cadence_computer() {
    // TODO: Add some harder test cases

    // 1740568588-2025-02-26T11:16:28
    let now = 1740568588;

    // next 5-minute boundary
    let (year, month, day, hour, minute, second) =
        unix_time_components(time_at_next_cadence_point(now, Dur::Minutes(5)));
    assert!(year == 2025);
    assert!(month == 1);
    assert!(day == 25);
    assert!(hour == 11);
    assert!(minute == 20);
    assert!(second == 0);

    // next 15-second boundary
    let (year, month, day, hour, minute, second) =
        unix_time_components(time_at_next_cadence_point(now, Dur::Seconds(15)));
    assert!(hour == 11);
    assert!(minute == 16);
    assert!(second == 30);

    // next 2-hour boundary
    let (year, month, day, hour, minute, second) =
        unix_time_components(time_at_next_cadence_point(now, Dur::Hours(2)));
    assert!(hour == 12);
    assert!(minute == 00);
    assert!(second == 00);

    // next 24-hour boundary is just next midnight
    let (year, month, day, hour, minute, second) =
        unix_time_components(time_at_next_cadence_point(now, Dur::Hours(24)));
    assert!(year == 2025);
    assert!(month == 1);
    assert!(day == 26);
    assert!(hour == 00);
    assert!(minute == 00);
    assert!(second == 00);

    let (year, month, day, hour, minute, second) =
        unix_time_components(time_at_next_cadence_point(now, Dur::Hours(48)));
    assert!(year == 2025);
    assert!(month == 1);
    assert!(day == 26);
    assert!(hour == 00);
    assert!(minute == 00);
    assert!(second == 00);

    let (year, month, day, hour, minute, second) =
        unix_time_components(time_at_next_cadence_point(now, Dur::Hours(72)));
    //println!("72: {year} {month} {day} {hour} {minute} {second}");
    assert!(year == 2025);
    assert!(month == 1);
    assert!(day == 27);
    assert!(hour == 00);
    assert!(minute == 00);
    assert!(second == 00);
}

///////////////////////////////////////////////////////////////////////////////////////////////////
//
//
// Facts about the kafka library.
//
// - Connections are reopened transparently if they time out, I think.
//
// - Connections to hosts are reused at a low level if they are not too old, creating a new client
//   is not necessarily a heavyweight operation involving lots of OS mechanics.
//
// - The consumer object needs to be polled to go look for messages.  When it does this it may hang
//   for a bit because it sends a message to the broker.  We can control this with
//   fetch_max_wait_time but it's unclear whether that applies only to the broker or if it is some
//   sort of network timeout control also - it mostly looks like the former.
//
// - We can choose whether to synchronously poll for control messages after sending a message, or
//   whether to do so only after sending a message *and* some time has elapsed, or whether to do so
//   regularly independently of sending the message.  In practice the computers running sonar will
//   be on continously and there's no great shame in polling once every minute or so, say (could be
//   a config control).  Or a combination.
//
// For now, we run an independent thread for the consumer, which will poll regularly (configurably)
// for control messages.

fn kafka_producer(receiver: mpsc::Receiver<(String,String)>, host: String) {
    // TODO: Error handling
    let mut producer =
        producer::Producer::from_hosts(vec!(host.to_owned()))
        .with_ack_timeout(Duration::from_secs(1))
        .with_required_acks(producer::RequiredAcks::One)
        .create()
        .unwrap();
    loop {
        let (topic, body) = receiver.recv().unwrap();
        //println!("sending: {topic} - {body}");
        producer.send(&producer::Record::from_value(&topic, body.as_bytes())).unwrap();
    }
}

fn kafka_consumer(sender: mpsc::Sender<Operation>, host: String, control_topic: String, poll_interval: Dur) {
    let mut consumer =
        consumer::Consumer::from_hosts(vec!(host.to_owned()))
        .with_topic(control_topic)
        .with_fallback_offset(consumer::FetchOffset::Latest)
        .with_group("".to_string())
        .create()
        .unwrap();
    loop {
        thread::sleep(std::time::Duration::from_secs(poll_interval.to_seconds()));
        for ms in consumer.poll().unwrap().iter() {
            'messages:
            for m in ms.messages() {
                // The protoc
                // https://docs.rs/kafka/0.10.0/kafka/client/fetch/struct.Message.html
                // the message has "key" and "value"
                // these are independent of the topic i think
                // to test
                //   $ ./kafka-console-producer.sh --bootstrap-server localhost:9092 --topic mlx.hpc.uio.no.node --property pars.key=true
                // and then use TAB to separate key and value on each line.
                let key = String::from_utf8_lossy(&m.key).to_string();
                let value = String::from_utf8_lossy(&m.value).to_string();
                match key.as_str() {
                    "exit" => {
                        sender.send(Operation::Control(ControlOp::Exit)).unwrap();
                    }
                    "dump" => {
                        let op: ControlOp;
                        match value.as_str() {
                            "true" => { op = ControlOp::DumpOn; }
                            "false" => { op = ControlOp::DumpOff; }
                            _ => { continue 'messages; }
                        }
                        sender.send(Operation::Control(op)).unwrap();
                    }
                    _ => {
                        continue 'messages;
                    }
                }
                //println!("{:?}", m);
            }
            consumer.consume_messageset(ms).unwrap();
        }
        // Not when not using a group
        //consumer.commit_consumed().unwrap();
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////
//
// Interrupts (may move elsewhere?)

fn interrupt_listener(_sender: mpsc::Sender<Operation>) {
    //todo!()
}

///////////////////////////////////////////////////////////////////////////////////////////////////
//
// Yet another config file parser.

fn parse_config(config_file: &str) -> Result<Ini, String> {
    let mut ini = Ini {
        global: GlobalIni {
            cluster: "".to_string(),
            role: "".to_string(),
            lockdir: None,
        },
        kafka: KafkaIni {
            remote_host: "".to_string(),
            poll_interval: Dur::Minutes(5),
        },
        debug: DebugIni {
            dump: false,
            verbose: false,
        },
        sample: SampleIni {
            cadence: None,
            exclude_system_jobs: true,
            load: true,
            batchless: false,
            exclude_commands: vec![],
            exclude_users: vec![],
        },
        sysinfo: SysinfoIni {
            on_startup: true,
            cadence: None,
        },
        slurm: SlurmIni {
            cadence: None,
            window: None,
        },
    };

    enum Section {
        None,
        Global,
        Kafka,
        Debug,
        Sample,
        Sysinfo,
        Slurm,
    }
    let mut curr_section = Section::None;
    let mut have_kafka = false;
    let mut have_kafka_remote = false;
    let file = match std::fs::File::open(config_file) {
        Ok(f) => f,
        Err(e) => { return Err(format!("{e}")); },
    };
    for l in std::io::BufReader::new(file).lines() {
        let l = match l {
            Ok(l) => l,
            Err(e) => { return Err(format!("{e}")); }
        };
        if l.starts_with('#') {
            continue;
        }
        let l = l.trim_ascii();
        if l.len() == 0 {
            continue;
        }
        if l == "[global]" {
            curr_section = Section::Global;
            continue;
        }
        if l == "[kafka]" {
            curr_section = Section::Kafka;
            have_kafka = true;
            continue;
        }
        if l == "[debug]" {
            curr_section = Section::Debug;
            continue;
        }
        if l == "[ps]" || l == "[sample]" {
            curr_section = Section::Sample;
            continue;
        }
        if l == "[sysinfo]" {
            curr_section = Section::Sysinfo;
            continue;
        }
        if l == "[slurm]" {
            curr_section = Section::Slurm;
            continue;
        }
        if l.starts_with("[") {
            return Err(format!("Unknown section {l}"))
        }

        let (name, value) = parse_setting(l)?;
        match curr_section {
            Section::None => {
                return Err("Setting outside section".to_string())
            }
            Section::Global => {
                match name.as_str() {
                    "cluster" => {
                        ini.global.cluster = value;
                    }
                    "role" => {
                        match value.as_str() {
                            "node" | "master" => {
                                ini.global.role = value;
                            }
                            _ => {
                                return Err(format!("Invalid global.role value `{value}`"))
                            }
                        }
                    }
                    "lockdir" => {
                        ini.global.lockdir = Some(value);
                    }
                    _ => {
                        return Err(format!("Invalid [global] setting name `{name}`"))
                    }
                }
            }
            Section::Kafka => {
                match name.as_str() {
                    "remote-host" => {
                        ini.kafka.remote_host = value;
                        have_kafka_remote = true;
                    }
                    "poll-interval" => {
                        ini.kafka.poll_interval = parse_duration(&value, false)?;
                    }
                    _ => {
                        return Err(format!("Invalid [kafka] setting name `{name}`"))
                    }
                }
            }
            Section::Debug => {
                match name.as_str() {
                    "dump" => {
                        ini.debug.dump = parse_bool(&value)?;
                    }
                    "verbose" => {
                        ini.debug.verbose = parse_bool(&value)?;
                    }
                    _ => {
                        return Err(format!("Invalid [debug] setting name `{name}`"))
                    }
                }
            }
            Section::Sample => {
                match name.as_str() {
                    "cadence" => {
                        ini.sample.cadence = Some(parse_duration(&value, false)?);
                    }
                    "exclude-system-jobs" => {
                        ini.sample.exclude_system_jobs = parse_bool(&value)?;
                    }
                    "load" => {
                        ini.sample.load = parse_bool(&value)?;
                    }
                    "exclude-users" => {
                        ini.sample.exclude_users = parse_strings(&value)?;
                    }
                    "exclude-commands" => {
                        ini.sample.exclude_commands = parse_strings(&value)?;
                    }
                    "batchless" => {
                        ini.sample.batchless = parse_bool(&value)?;
                    }
                    _ => {
                        return Err(format!("Invalid [sample]/[ps] setting name `{name}`"))
                    }
                }
            }
            Section::Sysinfo => {
                match name.as_str() {
                    "on-startup" => {
                        ini.sysinfo.on_startup = parse_bool(&value)?;
                    }
                    "cadence" => {
                        ini.sysinfo.cadence = Some(parse_duration(&value, false)?);
                    }
                    _ => {
                        return Err(format!("Invalid [sysinfo] setting name `{name}`"))
                    }
                }
            }
            Section::Slurm => {
                match name.as_str() {
                    "cadence" => {
                        ini.slurm.cadence = Some(parse_duration(&value, false)?);
                        if ini.slurm.window.is_none() {
                            ini.slurm.window = Some(Dur::Seconds(2 * ini.slurm.cadence.unwrap().to_seconds()));
                        }
                    }
                    "window" => {
                        ini.slurm.window = Some(parse_duration(&value, true)?);
                    }
                    _ => {
                        return Err(format!("Invalid [slurm] setting name `{name}`"))
                    }
                }
            }
        }
    }

    if ini.global.cluster == "" {
        return Err("Missing global.cluster setting".to_string());
    }
    if ini.global.role == "" {
        return Err("Missing global.role setting".to_string());
    }

    if have_kafka && !have_kafka_remote {
        return Err("Missing kafka.remote-host setting".to_string());
    }

    Ok(ini)
}

fn parse_setting(l: &str) -> Result<(String,String), String> {
    if let Some((name, value)) = l.split_once('=') {
        let name = name.trim_ascii();
        // A little too lenient
        for c in name.chars() {
            if !(c >= 'A' && c <= 'Z' || c >= 'a' && c <= 'z' || c >= '0' && c <= '9' || c == '-' || c == '_') {
                return Err("Illegal character in name".to_string());
            }
        }
        let value = value.trim_ascii();
        if value == "" {
            return Err("Empty string must be quoted".to_string());
        }
        let quotes = &['\'', '"', '`'];
        let value = if value.starts_with(quotes) {
            let v = value.strip_prefix(quotes).unwrap();
            if let Some(v) = v.strip_suffix(quotes) {
                v
            } else {
                return Err("Mismatched quotes".to_string());
            }
        } else {
            value
        };
        Ok((name.to_string(), value.to_string()))
    } else {
        return Err("Illegal property definition".to_string());
    }
}

fn parse_bool(l: &str) -> Result<bool, String> {
    match l {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("Invalid boolean value {l}")),
    }
}

fn parse_duration(l: &str, lenient: bool) -> Result<Dur, String> {
    if let Some(hours) = l.strip_suffix(&['h','H']) {
        if let Ok(k) = hours.parse::<u64>() {
            if k > 0 && (lenient || 24 % k == 0 || k % 24 == 0) {
                return Ok(Dur::Hours(k))
            }
        }
        return Err("Bad duration".to_string());
    }
    if let Some(minutes) = l.strip_suffix(&['m','M']) {
        if let Ok(k) = minutes.parse::<u64>() {
            if k > 0 && (lenient || (k < 60 && 60 % k == 0)) {
                return Ok(Dur::Minutes(k))
            }
        }
        return Err("Bad duration".to_string());
    }
    if let Some(seconds) = l.strip_suffix(&['s','S']) {
        if let Ok(k) = seconds.parse::<u64>() {
            if k > 0 && (lenient || (k < 60 && 60 % k == 0)) {
                return Ok(Dur::Seconds(k))
            }
        }
        return Err("Bad duration".to_string());
    }
    return Err("Bad duration".to_string());
}

fn parse_strings(l: &str) -> Result<Vec<String>, String> {
    if l == "" {
        Ok(vec![])
    } else {
        Ok(l.split(',').map(|x| x.to_string()).collect::<Vec<String>>())
    }
}

#[test]
pub fn test_parser() {
    let (a, b) = parse_setting(" x-factor = 10 ").unwrap();
    assert!(a == "x-factor");
    assert!(b == "10");
    let (a, b) = parse_setting("X_fact0r=`10 + 20`").unwrap();
    assert!(a == "X_fact0r");
    assert!(b == "10 + 20");
    assert!(parse_bool("true") == Ok(true));
    assert!(parse_bool("false") == Ok(false));
    assert!(parse_strings("").unwrap().len() == 0);
    assert!(parse_strings("a,b").unwrap().len() == 2);
    assert!(parse_duration("30s", true).unwrap() == Dur::Seconds(30));
    assert!(parse_duration("10m", true).unwrap() == Dur::Minutes(10));
    assert!(parse_duration("6H", true).unwrap() == Dur::Hours(6));

    assert!(parse_setting("zappa").is_err());
    assert!(parse_setting("zappa = ").is_err());
    assert!(parse_setting("zappa = `abracadabra").is_err());
    assert!(parse_setting("zapp! = true").is_err());
    assert!(parse_bool("tru").is_err());
    assert!(parse_duration("35", true).is_err());
    assert!(parse_duration("12m35s", true).is_err());
    assert!(parse_duration("3H12M35X", true).is_err());

    let ini = parse_config("src/testdata/daemon-config.txt").unwrap();
    assert!(ini.global.cluster == "mlx.hpc.uio.no");
    assert!(ini.global.role == "node");
    assert!(ini.kafka.remote_host == "naic-monitor.uio.no:12345");
    assert!(ini.sample.cadence == Some(Dur::Minutes(5)));
    assert!(ini.sample.batchless);
    assert!(!ini.sample.load);
    assert!(ini.sysinfo.cadence == Some(Dur::Hours(24)));
    assert!(ini.slurm.cadence == Some(Dur::Hours(1)));
    assert!(ini.slurm.window == Some(Dur::Minutes(90)));
}
