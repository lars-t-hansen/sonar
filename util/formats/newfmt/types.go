// This is a machine-processable executable specification for the new JSON format for Sonar output.
//
// Comments that are triple-slashed '///' will be extracted and attached to the entity (type, field)
// following.  The special +preamble comment is emitted as a preamble.

///+preamble
///
/// ## Introduction
///
/// Five types of data are collected:
///
/// * job and process sample data
/// * node sample data
/// * job data
/// * node configuration data
/// * cluster data
///
/// The job and process sample data are collected frequently on every node and comprise information
/// about each running job and the resource use of the processes in the job, for all pertinent
/// resources (cpu, ram, gpu, gpu ram, power consumption, i/o).
///
/// The node sample data are also collected on every node, normally at the same time as the job and
/// process sample data, and comprise information about the overall use of resources on the node
/// independently of jobs and processes, to the extent these can't be derived from the job and
/// process sample data.
///
/// The job data are collected on a master node and comprise information about the job that are not
/// directly related to moment-to-moment resource usage: start and end times, time in the queue,
/// allocation requests, completion status, billable resource use and so on.  (Only applies to
/// systems with a job manager / job queue.)
///
/// The node configuration data are collected occasionally on every node and comprise information
/// about the current configuration of the node.
///
/// The cluster data are collected occasionally on a master node and comprise information about the
/// cluster that are related to how nodes are grouped into partitions and short-term node status; it
/// complements node configuration data.
///
/// NOTE: Nodes may be added to clusters simply by submitting data for them.
///
/// NOTE: We do not yet collect some interesting cluster configuration data – how nodes, racks,
/// islands are connected and by what type of interconnect; the type of attached I/O.  Clusters are
/// added to the database through other APIs.
///
/// ## Data format overall notes
///
/// The output is a tree structure that is constrained enough to be serialized as
/// [JSON](https://www.rfc-editor.org/rfc/rfc8259) and other likely serialization formats (protobuf,
/// bson, cbor, a custom format, whatever).  It shall follow the [json:api
/// specification](https://jsonapi.org/format/#document-structure).  It generally does not
/// incorporate many size optimizations.
///
/// It's not a goal to have completely normalized data; redundancies are desirable in some cases to
/// make data self-describing.
///
/// In a serialization format that allows fields to be omitted, all fields except union fields will
/// have default values, which are zero, empty string, false, the empty object, or the empty array.
/// A union field must have exactly one member present.
///
/// Field values are constrained by data types described below, and sometimes by additional
/// constraints described in prose.  Primitive types are as they are in Go: 64-bit integers and
/// floating point, and Unicode strings.  Numeric values outside the given ranges, non-Unicode
/// string encodings, malformed timestamps, malformed node-ranges or type-incorrect data in any
/// field can cause the entire top-level object containing them to be rejected by the back-end.
///
/// The word "current" in the semantics of a field denotes an instantaneous reading or a
/// short-interval statistical measure; contrast "cumulative", which is since start of process/job
/// or since system boot or some other fixed time.
///
/// Field names generally do not carry unit information.  The units are included in the field
/// descriptions, but if they are not then they should be kilobytes for memory, megahertz for
/// clocks, watts for power, and percentage points for relative utilization measures.  (Here,
/// Kilobyte (KB) = 2^10, Megabyte (MB) = 2^20, Gigabyte (GB) = 2^30 bytes, SI notwithstanding.)
///
/// The top-level object for each output data type is the "...Envelope" object.
///
/// Within each envelope, `Data` and `Errors` are exclusive of each other.
///
/// Within each data object, no json:api `id` field is needed since the monitoring component is a
/// client in spec terms.
///
/// The errors field in an envelope is populated only for hard errors that prevent output from being
/// produced at all. Soft/recoverable errors are represented in the primary data objects.
///
/// MetadataObject and ErrorObject are shared between the various data types, everything else is
/// specific to the data type and has a name that clearly indicates that.
///
/// Some fields present in the older Sonar data are no longer here, having been deemed redundant or
/// obsolete.  Some are here in a different form.  Therefore, while old and new data are broadly
/// compatible, there may be some minor problems translating between them.
///
/// If a device does not expose a UUID, one will be constructed for it by the monitoring component.
/// This UUID will never be confusable with another device but it may change, eg at reboot, creating
/// a larger population of devices than there is in actuality.
///
/// ## Data format versions
///
/// This document describes data format version "0".  Adding fields or removing fields where default
/// values indicate missing values in the data format do not change the version number: the version
/// number only needs change if semantics of existing fields change in some incompatible way.  We
/// intend that the version will "never" change.

package newfmt

import (
	"errors"
)

/// String value where an empty value is an error, not simply absence of data
type NonemptyString string

/// Uint64 value where zero is an error, not simply absence of datga
type NonzeroUint uint64

/// RFC3999 localtime+TZO with no sub-second precision: yyyy-mm-ddThh:mm:ss+hh:mm, "Z" for +00:00.
type Timestamp NonemptyString

/// Timestamp, or empty string for missing data
type OptionalTimestamp string

/// Dotted host name or prefix of same, with standard restrictions on character set.
type Hostname NonemptyString

/// An unsigned value that carries two additional values: unset and infinite.  It has a more limited
/// value range than regular unsigned.
type ExtendedUint int64

// TODO: We want to expose these in the rendered spec, or include the information in the comment above.
const (
	ExtendedUintUnset    int64 = 0
	ExtendedUintInfinite int64 = -1
)

func (e ExtendedUint) ToUint() (uint64, error) {
	if e > 0 {
		return uint64(e - 1), nil
	}
	return 0, errors.New("Not a numeric value")
}

/// String-valued enum tag for the record type
type DataType NonemptyString

// TODO: Use these or get rid of them?
const (
	DTSample  DataType = "sample"
	DTSysinfo DataType = "sysinfo"
	DTJobs    DataType = "job"
	DTCluster DataType = "cluster"
)

/// Information about the data producer and the data format.  After the data have been ingested, the
/// metadata can be thrown away and will not affect the data contents.
///
/// NOTE: The `attrs` field can be used to transmit information about how the data were collected.
/// For example, sometimes Sonar is run with switches that exclude system jobs and short-running
/// jobs, and the data could record this.  For some agents, it may be desirable to report on eg
/// Python version (slurm-monitor does this).

type MetadataObject struct {
	/// The name of the component that generated the data (eg "sonar", "slurm-monitor")
	Producer NonemptyString `json:"producer"`

	/// The semver of the producer
	Version  NonemptyString `json:"version"`

	/// The data format version
	Format   uint64 `json:"format"`

	/// An array of generator-dependent attribute values
	Attrs    []KVPair `json:"attrs"`

	/// EXPERIMENTAL / UNDERSPECIFIED.  An API token to be used with
	/// Envelope.Data.Attributes.Cluster, it proves that the producer of the datum was authorized to
	/// produce data for that cluster name.
	Token string `json:"token"`
}

/// Information about a continuable or non-continuable error.

type ErrorObject struct {
	/// Time when the error was generated
	Time    Timestamp `json:"time"`

	/// A sensible English-language error message describing the error
	Detail  NonemptyString    `json:"detail"`

	/// Canonical cluster name for node generating the error
	Cluster Hostname  `json:"cluster"`

	/// name of node generating the error
	Node    Hostname  `json:"node"`
}

/// Carrier of arbitrary attribute data

type KVPair struct {
	/// A unique key within the array for the attribute
	Key   NonemptyString `json:"key"`

	/// Some attribute value
	Value string `json:"value"`
}

/// The Sysinfo object carries hardware information about a node.
///
/// NOTE: "Nodeinfo" would have been a better name but by now "Sysinfo" is baked into everything.
///
/// NOTE: Also see notes about envelope objects in the preamble.
///
/// NOTE: These are extracted from the node periodically, currently with Sonar we extract
/// information every 24h and on node boot.
///
/// NOTE: In the Go code, the JSON representation can be read with ConsumeJSONSysinfo().

type SysinfoEnvelope struct {
	/// Information about the producer and data format
	Meta   MetadataObject `json:"meta"`

	/// Node data, for successful probes
	Data   *SysinfoData   `json:"data"`

	/// Error information, for unsuccessful probes
	Errors []ErrorObject  `json:"errors"`
}

/// System data, for successful sysinfo probes

type SysinfoData struct {
	/// Data tag: The value "sysinfo"
	Type       DataType          `json:"type"`

	/// The node data themselves
	Attributes SysinfoAttributes `json:"attributes"`
}

/// This object describes a node, its CPUS, devices, topology and software
///
/// For the time being, we assume all cores on a node are the same. This is complicated by eg
/// BIG.little systems (performance cores vs efficiency cores), for one thing, but that's OK.
///
/// NOTE: The node may or may not be under control of Slurm or some other batch system.
/// However, that information is not recorded with the node information, but with the node sample
/// data, as the node can be added to or removed from a Slurm partition at any time.
///
/// NOTE: The number of physical cores is sockets * cores_per_socket.
///
/// NOTE: The number of logical cores is sockets * cores_per_socket * threads_per_core.

type SysinfoAttributes struct {
	/// Time the current data were obtained
	Time           Timestamp `json:"time"`

	/// The canonical cluster name
	Cluster        Hostname `json:"cluster"`

	/// The name of the host as it is known to itself
	Node           Hostname `json:"node"`

	/// Operating system name (the `sysname` field of `struct utsname`)
	OsName         NonemptyString `json:"os_name"`

	/// Operating system version (the `release` field of `struct utsname`)
	OsRelease      NonemptyString `json:"os_release"`

	/// Architecture name (the `machine` field of `struct utsname`)
	Architecture   NonemptyString                   `json:"architecture"`

	/// Number of CPU sockets
	Sockets        NonzeroUint                   `json:"sockets"`

	/// Number of physical cores per socket
	CoresPerSocket NonzeroUint                   `json:"cores_per_socket"`

	/// Number of hyperthreads per physical core
	ThreadsPerCore NonzeroUint                   `json:"threads_per_core"`

	/// Manufacturer's model name
	CpuModel       string                   `json:"cpu_model"`

	/// Primary memory in kilobytes
	Memory         NonzeroUint                   `json:"memory"`

	/// Base64-encoded SVG output of `lstopo`
	TopoSVG        string                   `json:"topo_svg"`

	/// Per-card information
	Cards          []SysinfoGpuCard         `json:"cards"`

	/// Per-software-package information
	Software       []SysinfoSoftwareVersion `json:"software"`
}

/// Per-card information.
///
/// NOTE: Many of the string values are idiosyncratic or have card-specific formats, and some are
/// not available on all cards.
///
/// NOTE: Only the UUID, manufacturer, model and architecture are required to be stable over time
/// (in practice, memory might be stable too).
///
/// NOTE: Though the power limit can change, it is reported here (as well as in sample data) because
/// it usually does not.

type SysinfoGpuCard struct {
	/// Local card index, may change at boot
	Index         uint64 `json:"index"`

	/// UUID as reported by card.  See notes in preamble
	UUID          string `json:"uuid"`

	/// Indicates an intra-system card address, eg PCI address
	Address       string `json:"address"`

	/// A keyword, "NVIDIA", "AMD", "Intel" (others TBD)
	Manufacturer  string `json:"manufacturer"`

	/// Card-dependent, this is the manufacturer's model string
	Model         string `json:"model"`

	/// Card-dependent, for NVIDIA this is "Turing", "Volta" etc
	Architecture  string `json:"architecture"`

	/// Card-dependent, the manufacturer's driver string
	Driver        string `json:"driver"`

	/// Card-dependent, the manufacturer's firmware string
	Firmware      string `json:"firmware"`

	/// GPU memory in kilobytes
	Memory        uint64 `json:"memory"`

	/// Power limit in watts
	PowerLimit    uint64 `json:"power_limit"`

	/// Max power limit in watts
	MaxPowerLimit uint64 `json:"max_power_limit"`

	/// Min power limit in watts
	MinPowerLimit uint64 `json:"min_power_limit"`

	/// Max clock of compute element
	MaxCEClock    uint64 `json:"max_ce_clock"`

	/// Max clock of GPU memory
	MaxMemoryClock uint64 `json:"max_memory_clock"`
}

/// The software versions are obtained by system-dependent means. As the monitoring component runs
/// outside the monitored processes' contexts and is not aware of software that has been loaded with
/// eg module load, the software reported in the software fields is thus software that is either
/// always loaded and always available to all programs, or which can be loaded by any program but
/// may or may not be.
///
/// NOTE: For GPU software: On NVIDIA systems, one can look in $CUDA_ROOT/version.json, where the
/// key/name/version values are encoded directly.  On AMD systems, one can look in
/// $ROCm_ROOT/.info/.version*, where the file name encodes the component key and the file stores
/// the version number. Clearly other types of software could also be reported for the node (R,
/// Jupyter, etc), based on information from modules, say.

type SysinfoSoftwareVersion struct {
	/// A unique identifier for the software package
	Key     NonemptyString `json:"key"`

	/// Human-readable name of the software package
	Name    string `json:"name"`

	/// The package's version number, in some package-specific format
	Version NonemptyString `json:"version"`
}

/// The "sample" record is sent from each node at each sampling interval in the form of a top-level
/// sample object.
///
/// NOTE: Also see notes about envelope objects in the preamble.
///
/// NOTE: JSON representation can be read with ConsumeJSONSamples().

type SampleEnvelope struct {
	/// Information about the producer and data format
	Meta   MetadataObject `json:"meta"`

	/// Sample data, for successful probes
	Data   *SampleData    `json:"data"`

	/// Error information, for unsuccessful probes
	Errors []ErrorObject  `json:"errors"`
}

/// Sample data, for successful sysinfo probes

type SampleData struct {
	/// Data tag: The value "sample"
	Type       DataType         `json:"type"`

	/// The sample data themselves
	Attributes SampleAttributes `json:"attributes"`
}

/// Holds the state of the node and the state of its running processes at a point in time, possibly
/// filtered.
///
/// NOTE: A SampleAttributes object with an empty jobs array represents a heartbeat from an idle
/// node, or a recoverable error situation if errors is not empty.

type SampleAttributes struct {
	/// Time the current data were obtained
	Time    Timestamp     `json:"time"`

	/// The canonical cluster name whence the datum originated
	Cluster Hostname      `json:"cluster"`

	/// The name of the node as it is known to the node itself
	Node    Hostname      `json:"node"`

	/// State of the node as a whole
	System  SampleSystem  `json:"system"`

	/// State of jobs on the nodes
	Jobs    []SampleJob   `json:"jobs"`

	/// Recoverable errors, if any
	Errors  []ErrorObject `json:"errors"`
}

/// This object describes the state of the node independently of the jobs running on it.
///
/// NOTE: Other node-wide fields will be added (e.g. for other load averages, additional memory
/// measures, for I/O and for energy).
///
/// NOTE: The sysinfo for the node provides the total memory; available memory = total - used.

type SampleSystem struct {
	/// The state of individual cores
	Cpus       []SampleCpu `json:"cpus"`

	/// The state of individual GPU devices
	Gpus       []SampleGpu `json:"gpus"`

	/// The amount of primary memory in use in kilobytes
	UsedMemory uint64      `json:"used_memory"`
}

/// The number of CPU seconds used by the core since boot.

type SampleCpu uint64

/// This object exposes utilization figures for the card.
///
/// NOTE: In all monitoring data, cards are identified both by current index and by immutable UUID,
/// this is redundant but hopefully useful.
///
/// NOTE: A card index may be local to a job, as Slurm jobs partition the system and may remap cards
/// to a local name space.  UUID is usually safer.
///
/// NOTE: Some fields are available on some cards and not on others.
///
/// NOTE: If there are multiple fans and we start caring about that then we can add a new field, eg
/// "fans", that holds an array of fan speed readings. Similarly, if there are multiple temperature
/// sensors and we care about that we can introduce a new field to hold an array of readings.

type SampleGpu struct {
	/// Local card index, may change at boot
	Index            uint64 `json:"index"`

	/// Card UUID.  See preamble for notes about UUIDs.
	UUID             NonemptyString `json:"uuid"`

	/// If not zero, an error code indicating a card failure state. code=1 is "generic failure".
	/// Other codes TBD.
	Failing          uint64 `json:"failing"`

	/// Percent of primary fan's max speed, may exceed 100% on some cards in some cases
	Fan              uint64 `json:"fan"`

	/// Current compute mode, completely card-specific if known at all
	ComputeMode      string `json:"compute_mode"`

	/// Current performance level, card-specific >= 0, or unset for "unknown".
	PerformanceState ExtendedUint   `json:"performance_state"`

	/// Memory use in Kilobytes
	Memory           uint64 `json:"memory"`

	/// Percent of computing element capability used
	CEUtil           uint64 `json:"ce_util"`

	/// Percent of memory used
	MemoryUtil       uint64 `json:"memory_util"`

	/// Degrees C card temperature at primary sensor (note can be negative)
	Temperature      int64  `json:"temperature"`

	/// Watts current power usage
	Power            uint64 `json:"power"`

	/// Watts current power limit
	PowerLimit       uint64 `json:"power_limit"`

	/// Compute element current clock
	CEClock          uint64 `json:"ce_clock"`

	/// memory current clock
	MemoryClock      uint64 `json:"memory_clock"`
}

/// Sample data for a single job
///
/// NOTE: Information about processes comes from various sources, and not all paths reveal all the
/// information, hence there is some hedging about what values there can be, eg for user names.
///
/// NOTE: The (job,epoch) pair must always be used together. If epoch is 0 then job is never 0 and
/// other (job,0) records coming from the same or other nodes in the same cluster at the same or
/// different time denote other aspects of the same job. Slurm jobs will have epoch=0, allowing us
/// to merge event streams from the job both intra- and inter-node, while non-mergeable jobs will
/// have epoch not zero. See extensive discussion in the "Rectification" section below.
///
/// NOTE: Other job-wide / cross-process / per-slurm-job fields can be added, e.g. for I/O and
/// energy, but only those that can only be monitored from within the node itself. Job data that can
/// be extracted from a Slurm master node will be sent with the job data, see later.
///
/// NOTE: Process entries can also be created for jobs running in containers. See below for
/// comments about the data that can be collected.
///
/// NOTE: On batch systems there may be more jobs than those managed by the batch system.
/// These are distinguished by a non-zero epoch, see above.

type SampleJob struct {
	/// The job ID
	Job       uint64          `json:"job"`

	/// User name on the cluster; `_user_<uid>` if not determined but user ID is available,
	/// `_user_unknown` otherwise.
	User      NonemptyString          `json:"user"`

	/// Zero for batch jobs, otherwise is a nonzero value that increases (by some amount) when the
	/// system reboots, and never wraps around. You may think of it as a boot counter for the node,
	/// but you must not assume that the values observed will be densely packed.  See notes.
	Epoch     uint64          `json:"epoch"`

	/// Processes in the job, all have the same Job ID.
	Processes []SampleProcess `json:"processes"`
}

/// Sample values for a single process within a job.
///
/// NOTE: Other per-process fields can be added, eg for I/O and energy.
///
/// NOTE: Memory utilization, produced by slurm-monitor, can be computed as resident_memory/memory
/// where resident_memory is the field above and memory is that field in the sysinfo object for the
/// node or in the slurm data (allocated memory).
///
/// NOTE: Resident memory is a complicated figure. What we want is probably the Pss ("Proportional
/// Set Size") which is private memory + a share of memory shared with other processes but that is
/// often not available. Then we must choose from just private memory (RssAnon) or private memory +
/// all resident memory shared with other processes (Rss). The former is problematic because it
/// undercounts memory, the latter problematic because summing resident memory of the processes will
/// frequently lead to a number that is more than physical memory as shared memory is counted
/// several times.
///
/// NOTE: Container software may not reveal all the process data we want. Docker, for example,
/// provides cpu_util but not cpu_avg or cpu_time, and a memory utilization figure from which
/// resident_memory must be back-computed.
///
/// NOTE: The fields cpu_time, cpu_avg, and cpu_util are different views on the same
/// quantities and are used variously by Sonar and the slurm-monitor dashboard. The Jobanalyzer
/// back-end computes its own cpu_util from a time series of cpu_time values and using the
/// cpu_avg as the first value in the computed series. The slurm-monitor dashboard in contrast
/// uses cpu_util directly, but as it will require some time to perform the sampling it slows down
/// the monitoring process (a little) and make it more expensive (a little), and the result is less
/// accurate (it's a sample, not an averaging over the entire interval). Possibly having either
/// cpu_avg and cpu_time together or cpu_util on its own would be sufficient.
///
/// NOTE: `rolledup` is a Sonar data-compression feature that should probably be removed or
/// improved, as information is lost. It is employed only if sonar is invoked with --rollup.  At the
/// same time, for a node running 128 (say) MPI processes for the same job it represents a real
/// savings in data volume.

type SampleProcess struct {
	/// Kilobytes of private resident memory.
	ResidentMemory  uint64             `json:"resident_memory"`

	/// Kilobytes of virtual data+stack memory
	VirtualMemory   uint64             `json:"virtual_memory"`

	/// The command (not the command line), zombie processes get an extra <defunct> annotation at
	/// the end, a la ps.
	Cmd       string             `json:"cmd"`

	/// Process ID, zero is used for rolled-up processes.
	Pid       uint64             `json:"pid"`

	/// Parent-process ID.
	ParentPid uint64             `json:"ppid"`

	/// The running average CPU percentage over the true lifetime of the process as reported
	/// by the operating system. 100.0 corresponds to "one full core's worth of computation".
	/// See notes.
	CpuAvg    float64            `json:"cpu_avg"`

	/// The current sampled CPU utilization of the process, 100.0 corresponds to "one full core's
	/// worth of computation". See notes.
	CpuUtil   float64            `json:"cpu_util"`

	/// Cumulative CPU time in seconds for the process over its lifetime. See notes.
	CpuTime   uint64             `json:"cpu_time"`

	/// The number of additional processes in the same cmd and no child processes that have been
	/// rolled into this one. That is, if the value is 1, the record represents the sum of the data
	/// for two processes.
	Rolledup  int                `json:"rolledup"`

	/// GPU sample data for all cards used by the process.
	Gpus      []SampleProcessGpu `json:"gpus"`
}

/// Per-process per-gpu sample data.
///
/// NOTE: The difference between gpu_memory and gpu_memory_util is that, on some cards some of the
/// time, it is possible to determine one of these but not the other, and vice versa. For example,
/// on the NVIDIA cards we can read both quantities for running processes but only gpu_memory for
/// some zombies. On the other hand, on our AMD cards there used to be no support for detecting the
/// absolute amount of memory used, nor the total amount of memory on the cards, only the percentage
/// of gpu memory used (gpu_memory_util). Sometimes we can convert one figure to another, but other
/// times we cannot quite do that. Rather than encoding the logic for dealing with this in the
/// monitoring component, the task is currently offloaded to the back end. It would be good to clean
/// this up, with experience from more GPU types too - maybe gpu_memory_util can be removed.
///
/// NOTE: Some cards do not reveal the amount of compute or memory per card per process, only which
/// cards and how much compute or memory in aggregate (NVIDIA at least provides the more detailed
/// data). In that case, the data revealed here for each card will be the aggregate figure for the
/// process divided by the number of cards the process is running on.

type SampleProcessGpu struct {
	/// Local card index, may change at boot
	Index            uint64 `json:"index"`

	/// Card UUID.  See preamble for notes about UUIDs.
	UUID             NonemptyString `json:"uuid"`

	/// The current GPU percentage utilization for the process on the card.
	GpuUtil    float64 `json:"gpu_util"`

	/// The current GPU memory used in kilobytes for the process on the card. See notes.
	GpuMemory     uint64  `json:"gpu_memory"`

	/// The current GPU memory usage percentage for the process on the card. See notes.
	GpuMemoryUtil float64 `json:"gpu_memory_util"`
}

// JSON representation can be read with ConsumeJSONJobs().

type JobsEnvelope struct {
	Data   *JobsData      `json:"data"`
	Errors []ErrorObject  `json:"errors"`
	Meta   MetadataObject `json:"meta"`
}

type JobsData struct {
	Type       DataType       `json:"type"` // DTJobs
	Attributes JobsAttributes `json:"attributes"`
}

type JobsAttributes struct {
	Time    Timestamp `json:"time"`
	Cluster Hostname  `json:"cluster"`
	// There can eventually be other types of jobs, there will be other fields for them here, and
	// the decoder will populate the correct field.  Other fields will be nil.
	SlurmJobs []SlurmJob `json:"slurm_jobs"`
}

// This follows the order of the spec (at the time I write this).  Fields with substructure
// (AllocTRES, GRESDetail) may have parsers, see other files in this package.

type SlurmJob struct {
	JobID          uint64    `json:"job_id"`
	JobName        string    `json:"job_name"`
	JobState       string    `json:"job_state"`
	JobStep        string    `json:"job_step"`
	ArrayJobID     uint64    `json:"array_job_id"`
	ArrayTaskID    uint64    `json:"array_task_id"`
	HetJobID       uint64    `json:"het_job_id"`
	HetJobOffset   uint64    `json:"het_job_offset"`
	UserName       string    `json:"user_name"`
	Account        string    `json:"account"`
	SubmitTime     Timestamp `json:"submit_time"`
	Timelimit      ExtendedUint      `json:"time_limit"`
	Partition      string    `json:"partition"`
	Reservation    string    `json:"reservation"`
	NodeList       []string  `json:"nodes"`
	Priority       ExtendedUint      `json:"priority"`
	Layout         string    `json:"distribution"`
	GRESDetail     []string  `json:"gres_detail"`
	ReqCPUS        uint64    `json:"requested_cpus"`
	ReqMemoryPerNode uint64    `json:"requested_memory_per_node"`
	ReqNodes       uint64    `json:"requested_node_count"`
	MinCPUSPerNode uint64    `json:"minimum_cpus_per_node"`
	Start          Timestamp `json:"start_time"`
	Suspended      uint64    `json:"suspend_time"`
	End            Timestamp `json:"end_time"`
	ExitCode       uint64    `json:"exit_code"`
	Sacct          *SacctData `json:"sacct"`
}

type SacctData struct {
	MinCPU       uint64 `json:"MinCPU"`
	AllocTRES    string `json:"AllocTRES"`
	AveCPU       uint64 `json:"AveCPU"`
	AveDiskRead  uint64 `json:"AveDiskRead"`
	AveDiskWrite uint64 `json:"AveDiskWrite"`
	AveRSS       uint64 `json:"AveRSS"`
	AveVMSize    uint64 `json:"AveVMSize"`
	ElapsedRaw   uint64 `json:"ElapsedRaw"`
	SystemCPU    uint64 `json:"SystemCPU"`
	UserCPU      uint64 `json:"UserCPU"`
	MaxRSS       uint64 `json:"MaxRSS"`
	MaxVMSize    uint64 `json:"MaxVMSize"`
}

// JSON representation can be read with ConsumeJSONCluster().

type ClusterEnvelope struct {
	Data   *ClusterData   `json:"data"`
	Errors []ErrorObject  `json:"errors"`
	Meta   MetadataObject `json:"meta"`
}

type ClusterData struct {
	Type       DataType          `json:"type"` // DTCluster
	Attributes ClusterAttributes `json:"attributes"`
}

// `Slurm` is set if at least some nodes and jobs are managed by Slurm.  All clusters are assumed to
// have some unmanaged jobs.

type ClusterAttributes struct {
	Time       Timestamp          `json:"time"`
	Cluster    Hostname           `json:"cluster"`
    Slurm      bool               `json:"slurm"`
	Partitions []ClusterPartition `json:"partitions"`
	Nodes      []ClusterNodes     `json:"nodes"`
}

type ClusterPartition struct {
	Name  string      `json:"name"`
	Nodes []NodeRange `json:"nodes"`
}

// Node state depends on the cluster type.  For Slurm, see sinfo(1), it's a long list.

type ClusterNodes struct {
	Names  []NodeRange `json:"names"`
	States []string    `json:"states"`
}

/// A nonempty-string representing a list of hostnames compactly using a simple syntax: brackets
/// introduce a list of individual numbered nodes and ranges, these are expanded to yield a list of
/// node names.  For example, `c[1-3,5]-[2-4].fox` yields `c1-2.fox`, `c1-3.fox`, `c1-4.fox`,
/// `c2-2.fox`, `c2-3.fox`, `c2-4.fox`, `c3-2.fox`, `c3-3.fox`, `c3-4.fox`, `c5-2.fox`, `c5-3.fox`,
/// `c5-4.fox`.  In a valid range, the first number is no greater than the second number, and
/// numbers are not repeated.  (The motivation for this feature is that some clusters have very many
/// nodes and that they group well this way.)

type NodeRange NonemptyString
