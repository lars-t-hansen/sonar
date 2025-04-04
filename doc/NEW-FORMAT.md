# Sonar JSON format output specification


## Introduction

Five types of data are collected:

* job and process sample data
* node sample data
* job data
* node configuration data
* cluster data

The job and process sample data are collected frequently on every node and comprise information
about each running job and the resource use of the processes in the job, for all pertinent
resources (cpu, ram, gpu, gpu ram, power consumption, i/o).

The node sample data are also collected on every node, normally at the same time as the job and
process sample data, and comprise information about the overall use of resources on the node
independently of jobs and processes, to the extent these can't be derived from the job and
process sample data.

The job data are collected on a master node and comprise information about the job that are not
directly related to moment-to-moment resource usage: start and end times, time in the queue,
allocation requests, completion status, billable resource use and so on.  (Only applies to
systems with a job manager / job queue.)

The node configuration data are collected occasionally on every node and comprise information
about the current configuration of the node.

The cluster data are collected occasionally on a master node and comprise information about the
cluster that are related to how nodes are grouped into partitions and short-term node status; it
complements node configuration data.

NOTE: Nodes may be added to clusters simply by submitting data for them.

NOTE: We do not yet collect some interesting cluster configuration data – how nodes, racks,
islands are connected and by what type of interconnect; the type of attached I/O.  Clusters are
added to the database through other APIs.

## Data format overall notes

The output is a tree structure that is constrained enough to be serialized as
[JSON](https://www.rfc-editor.org/rfc/rfc8259) and other likely serialization formats (protobuf,
bson, cbor, a custom format, whatever).  It shall follow the [json:api
specification](https://jsonapi.org/format/#document-structure).  It generally does not
incorporate many size optimizations.

It's not a goal to have completely normalized data; redundancies are desirable in some cases to
make data self-describing.

In a serialization format that allows fields to be omitted, all fields except union fields will
have default values, which are zero, empty string, false, the empty object, or the empty array.
A union field must have exactly one member present.

Field values are constrained by data types described below, and sometimes by additional
constraints described in prose.  Primitive types are as they are in Go: 64-bit integers and
floating point, and Unicode strings.  Numeric values outside the given ranges, non-Unicode
string encodings, malformed timestamps, malformed node-ranges or type-incorrect data in any
field can cause the entire top-level object containing them to be rejected by the back-end.

The word "current" in the semantics of a field denotes an instantaneous reading or a
short-interval statistical measure; contrast "cumulative", which is since start of process/job
or since system boot or some other fixed time.

Field names generally do not carry unit information.  The units are included in the field
descriptions, but if they are not then they should be kilobytes for memory, megahertz for
clocks, watts for power, and percentage points for relative utilization measures.  (Here,
Kilobyte (KB) = 2^10, Megabyte (MB) = 2^20, Gigabyte (GB) = 2^30 bytes, SI notwithstanding.)

The top-level object for each output data type is the "...Envelope" object.

Within each envelope, `Data` and `Errors` are exclusive of each other.

Within each data object, no json:api `id` field is needed since the monitoring component is a
client in spec terms.

The errors field in an envelope is populated only for hard errors that prevent output from being
produced at all. Soft/recoverable errors are represented in the primary data objects.

MetadataObject and ErrorObject are shared between the various data types, everything else is
specific to the data type and has a name that clearly indicates that.

Some fields present in the older Sonar data are no longer here, having been deemed redundant or
obsolete.  Some are here in a different form.  Therefore, while old and new data are broadly
compatible, there may be some minor problems translating between them.

If a device does not expose a UUID, one will be constructed for it by the monitoring component.
This UUID will never be confusable with another device but it may change, eg at reboot, creating
a larger population of devices than there is in actuality.

## Data format versions

This document describes data format version "0".  Adding fields or removing fields where default
values indicate missing values in the data format do not change the version number: the version
number only needs change if semantics of existing fields change in some incompatible way.  We
intend that the version will "never" change.

## Data types

### Type: `NonemptyString`

String value where an empty value is an error, not simply absence of data

### Type: `NonzeroUint`

Uint64 value where zero is an error, not simply absence of datga

### Type: `Timestamp`

RFC3999 localtime+TZO with no sub-second precision: yyyy-mm-ddThh:mm:ss+hh:mm, "Z" for +00:00.

### Type: `OptionalTimestamp`

Timestamp, or empty string for missing data

### Type: `Hostname`

Dotted host name or prefix of same, with standard restrictions on character set.

### Type: `ExtendedUint`

An unsigned value that carries two additional values: unset and infinite.  It has a more limited
value range than regular unsigned.

### Type: `DataType`

String-valued enum tag for the record type

### Type: `MetadataObject`

Information about the data producer and the data format.  After the data have been ingested, the
metadata can be thrown away and will not affect the data contents.

NOTE: The `attrs` field can be used to transmit information about how the data were collected.
For example, sometimes Sonar is run with switches that exclude system jobs and short-running
jobs, and the data could record this.  For some agents, it may be desirable to report on eg
Python version (slurm-monitor does this).

#### **`producer`** NonemptyString

The name of the component that generated the data (eg "sonar", "slurm-monitor")

#### **`version`** NonemptyString

The semver of the producer

#### **`format`** uint64

The data format version

#### **`attrs`** []KVPair

An array of generator-dependent attribute values

#### **`token`** string

EXPERIMENTAL / UNDERSPECIFIED.  An API token to be used with
Envelope.Data.Attributes.Cluster, it proves that the producer of the datum was authorized to
produce data for that cluster name.

### Type: `ErrorObject`

Information about a continuable or non-continuable error.

#### **`time`** Timestamp

Time when the error was generated

#### **`detail`** NonemptyString

A sensible English-language error message describing the error

#### **`cluster`** Hostname

Canonical cluster name for node generating the error

#### **`node`** Hostname

name of node generating the error

### Type: `KVPair`

Carrier of arbitrary attribute data

#### **`key`** NonemptyString

A unique key within the array for the attribute

#### **`value`** string

Some attribute value

### Type: `SysinfoEnvelope`

The Sysinfo object carries hardware information about a node.

NOTE: "Nodeinfo" would have been a better name but by now "Sysinfo" is baked into everything.

NOTE: Also see notes about envelope objects in the preamble.

NOTE: These are extracted from the node periodically, currently with Sonar we extract
information every 24h and on node boot.

NOTE: In the Go code, the JSON representation can be read with ConsumeJSONSysinfo().

#### **`meta`** MetadataObject

Information about the producer and data format

#### **`data`** *SysinfoData

Node data, for successful probes

#### **`errors`** []ErrorObject

Error information, for unsuccessful probes

### Type: `SysinfoData`

System data, for successful sysinfo probes

#### **`type`** DataType

Data tag: The value "sysinfo"

#### **`attributes`** SysinfoAttributes

The node data themselves

### Type: `SysinfoAttributes`

This object describes a node, its CPUS, devices, topology and software

For the time being, we assume all cores on a node are the same. This is complicated by eg
BIG.little systems (performance cores vs efficiency cores), for one thing, but that's OK.

NOTE: The node may or may not be under control of Slurm or some other batch system.
However, that information is not recorded with the node information, but with the node sample
data, as the node can be added to or removed from a Slurm partition at any time.

NOTE: The number of physical cores is sockets * cores_per_socket.

NOTE: The number of logical cores is sockets * cores_per_socket * threads_per_core.

#### **`time`** Timestamp

Time the current data were obtained

#### **`cluster`** Hostname

The canonical cluster name

#### **`node`** Hostname

The name of the host as it is known to itself

#### **`os_name`** NonemptyString

Operating system name (the `sysname` field of `struct utsname`)

#### **`os_release`** NonemptyString

Operating system version (the `release` field of `struct utsname`)

#### **`architecture`** NonemptyString

Architecture name (the `machine` field of `struct utsname`)

#### **`sockets`** NonzeroUint

Number of CPU sockets

#### **`cores_per_socket`** NonzeroUint

Number of physical cores per socket

#### **`threads_per_core`** NonzeroUint

Number of hyperthreads per physical core

#### **`cpu_model`** string

Manufacturer's model name

#### **`memory`** NonzeroUint

Primary memory in kilobytes

#### **`topo_svg`** string

Base64-encoded SVG output of `lstopo`

#### **`cards`** []SysinfoGpuCard

Per-card information

#### **`software`** []SysinfoSoftwareVersion

Per-software-package information

### Type: `SysinfoGpuCard`

Per-card information.

NOTE: Many of the string values are idiosyncratic or have card-specific formats, and some are
not available on all cards.

NOTE: Only the UUID, manufacturer, model and architecture are required to be stable over time
(in practice, memory might be stable too).

NOTE: Though the power limit can change, it is reported here (as well as in sample data) because
it usually does not.

#### **`index`** uint64

Local card index, may change at boot

#### **`uuid`** string

UUID as reported by card.  See notes in preamble

#### **`address`** string

Indicates an intra-system card address, eg PCI address

#### **`manufacturer`** string

A keyword, "NVIDIA", "AMD", "Intel" (others TBD)

#### **`model`** string

Card-dependent, this is the manufacturer's model string

#### **`architecture`** string

Card-dependent, for NVIDIA this is "Turing", "Volta" etc

#### **`driver`** string

Card-dependent, the manufacturer's driver string

#### **`firmware`** string

Card-dependent, the manufacturer's firmware string

#### **`memory`** uint64

GPU memory in kilobytes

#### **`power_limit`** uint64

Power limit in watts

#### **`max_power_limit`** uint64

Max power limit in watts

#### **`min_power_limit`** uint64

Min power limit in watts

#### **`max_ce_clock`** uint64

Max clock of compute element

#### **`max_memory_clock`** uint64

Max clock of GPU memory

### Type: `SysinfoSoftwareVersion`

The software versions are obtained by system-dependent means. As the monitoring component runs
outside the monitored processes' contexts and is not aware of software that has been loaded with
eg module load, the software reported in the software fields is thus software that is either
always loaded and always available to all programs, or which can be loaded by any program but
may or may not be.

NOTE: For GPU software: On NVIDIA systems, one can look in $CUDA_ROOT/version.json, where the
key/name/version values are encoded directly.  On AMD systems, one can look in
$ROCm_ROOT/.info/.version*, where the file name encodes the component key and the file stores
the version number. Clearly other types of software could also be reported for the node (R,
Jupyter, etc), based on information from modules, say.

#### **`key`** NonemptyString

A unique identifier for the software package

#### **`name`** string

Human-readable name of the software package

#### **`version`** NonemptyString

The package's version number, in some package-specific format

### Type: `SampleEnvelope`

The "sample" record is sent from each node at each sampling interval in the form of a top-level
sample object.

NOTE: Also see notes about envelope objects in the preamble.

NOTE: JSON representation can be read with ConsumeJSONSamples().

#### **`meta`** MetadataObject

Information about the producer and data format

#### **`data`** *SampleData

Sample data, for successful probes

#### **`errors`** []ErrorObject

Error information, for unsuccessful probes

### Type: `SampleData`

Sample data, for successful sysinfo probes

#### **`type`** DataType

Data tag: The value "sample"

#### **`attributes`** SampleAttributes

The sample data themselves

### Type: `SampleAttributes`

Holds the state of the node and the state of its running processes at a point in time, possibly
filtered.

NOTE: A SampleAttributes object with an empty jobs array represents a heartbeat from an idle
node, or a recoverable error situation if errors is not empty.

#### **`time`** Timestamp

Time the current data were obtained

#### **`cluster`** Hostname

The canonical cluster name whence the datum originated

#### **`node`** Hostname

The name of the node as it is known to the node itself

#### **`system`** SampleSystem

State of the node as a whole

#### **`jobs`** []SampleJob

State of jobs on the nodes

#### **`errors`** []ErrorObject

Recoverable errors, if any

### Type: `SampleSystem`

This object describes the state of the node independently of the jobs running on it.

NOTE: Other node-wide fields will be added (e.g. for other load averages, additional memory
measures, for I/O and for energy).

NOTE: The sysinfo for the node provides the total memory; available memory = total - used.

#### **`cpus`** []SampleCpu

The state of individual cores

#### **`gpus`** []SampleGpu

The state of individual GPU devices

#### **`used_memory`** uint64

The amount of primary memory in use in kilobytes

### Type: `SampleCpu`

The number of CPU seconds used by the core since boot.

### Type: `SampleGpu`

This object exposes utilization figures for the card.

NOTE: In all monitoring data, cards are identified both by current index and by immutable UUID,
this is redundant but hopefully useful.

NOTE: A card index may be local to a job, as Slurm jobs partition the system and may remap cards
to a local name space.  UUID is usually safer.

NOTE: Some fields are available on some cards and not on others.

NOTE: If there are multiple fans and we start caring about that then we can add a new field, eg
"fans", that holds an array of fan speed readings. Similarly, if there are multiple temperature
sensors and we care about that we can introduce a new field to hold an array of readings.

#### **`index`** uint64

Local card index, may change at boot

#### **`uuid`** NonemptyString

Card UUID.  See preamble for notes about UUIDs.

#### **`failing`** uint64

If not zero, an error code indicating a card failure state. code=1 is "generic failure".
Other codes TBD.

#### **`fan`** uint64

Percent of primary fan's max speed, may exceed 100% on some cards in some cases

#### **`compute_mode`** string

Current compute mode, completely card-specific if known at all

#### **`performance_state`** ExtendedUint

Current performance level, card-specific >= 0, or unset for "unknown".

#### **`memory`** uint64

Memory use in Kilobytes

#### **`ce_util`** uint64

Percent of computing element capability used

#### **`memory_util`** uint64

Percent of memory used

#### **`temperature`** int64

Degrees C card temperature at primary sensor (note can be negative)

#### **`power`** uint64

Watts current power usage

#### **`power_limit`** uint64

Watts current power limit

#### **`ce_clock`** uint64

Compute element current clock

#### **`memory_clock`** uint64

memory current clock

### Type: `SampleJob`

Sample data for a single job

NOTE: Information about processes comes from various sources, and not all paths reveal all the
information, hence there is some hedging about what values there can be, eg for user names.

NOTE: The (job,epoch) pair must always be used together. If epoch is 0 then job is never 0 and
other (job,0) records coming from the same or other nodes in the same cluster at the same or
different time denote other aspects of the same job. Slurm jobs will have epoch=0, allowing us
to merge event streams from the job both intra- and inter-node, while non-mergeable jobs will
have epoch not zero. See extensive discussion in the "Rectification" section below.

NOTE: Other job-wide / cross-process / per-slurm-job fields can be added, e.g. for I/O and
energy, but only those that can only be monitored from within the node itself. Job data that can
be extracted from a Slurm master node will be sent with the job data, see later.

NOTE: Process entries can also be created for jobs running in containers. See below for
comments about the data that can be collected.

NOTE: On batch systems there may be more jobs than those managed by the batch system.
These are distinguished by a non-zero epoch, see above.

#### **`job`** uint64

The job ID

#### **`user`** NonemptyString

User name on the cluster; `_user_<uid>` if not determined but user ID is available,
`_user_unknown` otherwise.

#### **`epoch`** uint64

Zero for batch jobs, otherwise is a nonzero value that increases (by some amount) when the
system reboots, and never wraps around. You may think of it as a boot counter for the node,
but you must not assume that the values observed will be densely packed.  See notes.

#### **`processes`** []SampleProcess

Processes in the job, all have the same Job ID.

### Type: `SampleProcess`

Sample values for a single process within a job.

NOTE: Other per-process fields can be added, eg for I/O and energy.

NOTE: Memory utilization, produced by slurm-monitor, can be computed as resident_memory/memory
where resident_memory is the field above and memory is that field in the sysinfo object for the
node or in the slurm data (allocated memory).

NOTE: Resident memory is a complicated figure. What we want is probably the Pss ("Proportional
Set Size") which is private memory + a share of memory shared with other processes but that is
often not available. Then we must choose from just private memory (RssAnon) or private memory +
all resident memory shared with other processes (Rss). The former is problematic because it
undercounts memory, the latter problematic because summing resident memory of the processes will
frequently lead to a number that is more than physical memory as shared memory is counted
several times.

NOTE: Container software may not reveal all the process data we want. Docker, for example,
provides cpu_util but not cpu_avg or cpu_time, and a memory utilization figure from which
resident_memory must be back-computed.

NOTE: The fields cpu_time, cpu_avg, and cpu_util are different views on the same
quantities and are used variously by Sonar and the slurm-monitor dashboard. The Jobanalyzer
back-end computes its own cpu_util from a time series of cpu_time values and using the
cpu_avg as the first value in the computed series. The slurm-monitor dashboard in contrast
uses cpu_util directly, but as it will require some time to perform the sampling it slows down
the monitoring process (a little) and make it more expensive (a little), and the result is less
accurate (it's a sample, not an averaging over the entire interval). Possibly having either
cpu_avg and cpu_time together or cpu_util on its own would be sufficient.

NOTE: `rolledup` is a Sonar data-compression feature that should probably be removed or
improved, as information is lost. It is employed only if sonar is invoked with --rollup.  At the
same time, for a node running 128 (say) MPI processes for the same job it represents a real
savings in data volume.

#### **`resident_memory`** uint64

Kilobytes of private resident memory.

#### **`virtual_memory`** uint64

Kilobytes of virtual data+stack memory

#### **`cmd`** string

The command (not the command line), zombie processes get an extra <defunct> annotation at
the end, a la ps.

#### **`pid`** uint64

Process ID, zero is used for rolled-up processes.

#### **`ppid`** uint64

Parent-process ID.

#### **`cpu_avg`** float64

The running average CPU percentage over the true lifetime of the process as reported
by the operating system. 100.0 corresponds to "one full core's worth of computation".
See notes.

#### **`cpu_util`** float64

The current sampled CPU utilization of the process, 100.0 corresponds to "one full core's
worth of computation". See notes.

#### **`cpu_time`** uint64

Cumulative CPU time in seconds for the process over its lifetime. See notes.

#### **`rolledup`** int

The number of additional processes in the same cmd and no child processes that have been
rolled into this one. That is, if the value is 1, the record represents the sum of the data
for two processes.

#### **`gpus`** []SampleProcessGpu

GPU sample data for all cards used by the process.

### Type: `SampleProcessGpu`

Per-process per-gpu sample data.

NOTE: The difference between gpu_memory and gpu_memory_util is that, on some cards some of the
time, it is possible to determine one of these but not the other, and vice versa. For example,
on the NVIDIA cards we can read both quantities for running processes but only gpu_memory for
some zombies. On the other hand, on our AMD cards there used to be no support for detecting the
absolute amount of memory used, nor the total amount of memory on the cards, only the percentage
of gpu memory used (gpu_memory_util). Sometimes we can convert one figure to another, but other
times we cannot quite do that. Rather than encoding the logic for dealing with this in the
monitoring component, the task is currently offloaded to the back end. It would be good to clean
this up, with experience from more GPU types too - maybe gpu_memory_util can be removed.

NOTE: Some cards do not reveal the amount of compute or memory per card per process, only which
cards and how much compute or memory in aggregate (NVIDIA at least provides the more detailed
data). In that case, the data revealed here for each card will be the aggregate figure for the
process divided by the number of cards the process is running on.

#### **`index`** uint64

Local card index, may change at boot

#### **`uuid`** NonemptyString

Card UUID.  See preamble for notes about UUIDs.

#### **`gpu_util`** float64

The current GPU percentage utilization for the process on the card.

#### **`gpu_memory`** uint64

The current GPU memory used in kilobytes for the process on the card. See notes.

#### **`gpu_memory_util`** float64

The current GPU memory usage percentage for the process on the card. See notes.

### Type: `JobsEnvelope`


#### **`data`** *JobsData


#### **`errors`** []ErrorObject


#### **`meta`** MetadataObject


### Type: `JobsData`


#### **`type`** DataType


#### **`attributes`** JobsAttributes


### Type: `JobsAttributes`


#### **`time`** Timestamp


#### **`cluster`** Hostname


### Type: `SlurmJob`


#### **`job_id`** uint64


#### **`job_name`** string


#### **`job_state`** string


#### **`job_step`** string


#### **`array_job_id`** uint64


#### **`array_task_id`** uint64


#### **`het_job_id`** uint64


#### **`het_job_offset`** uint64


#### **`user_name`** string


#### **`account`** string


#### **`submit_time`** Timestamp


#### **`time_limit`** ExtendedUint


#### **`partition`** string


#### **`reservation`** string


#### **`nodes`** []string


#### **`priority`** ExtendedUint


#### **`distribution`** string


#### **`gres_detail`** []string


#### **`requested_cpus`** uint64


#### **`requested_memory_per_node`** uint64


#### **`requested_node_count`** uint64


#### **`minimum_cpus_per_node`** uint64


#### **`start_time`** Timestamp


#### **`suspend_time`** uint64


#### **`end_time`** Timestamp


#### **`exit_code`** uint64


#### **`sacct`** *SacctData


### Type: `SacctData`


#### **`MinCPU`** uint64


#### **`AllocTRES`** string


#### **`AveCPU`** uint64


#### **`AveDiskRead`** uint64


#### **`AveDiskWrite`** uint64


#### **`AveRSS`** uint64


#### **`AveVMSize`** uint64


#### **`ElapsedRaw`** uint64


#### **`SystemCPU`** uint64


#### **`UserCPU`** uint64


#### **`MaxRSS`** uint64


#### **`MaxVMSize`** uint64


### Type: `ClusterEnvelope`


#### **`data`** *ClusterData


#### **`errors`** []ErrorObject


#### **`meta`** MetadataObject


### Type: `ClusterData`


#### **`type`** DataType


#### **`attributes`** ClusterAttributes


### Type: `ClusterAttributes`


#### **`time`** Timestamp


#### **`cluster`** Hostname


#### **`slurm`** bool


#### **`partitions`** []ClusterPartition


#### **`nodes`** []ClusterNodes


### Type: `ClusterPartition`


#### **`name`** string


#### **`nodes`** []NodeRange


### Type: `ClusterNodes`


#### **`names`** []NodeRange


#### **`states`** []string


### Type: `NodeRange`

A nonempty-string representing a list of hostnames compactly using a simple syntax: brackets
introduce a list of individual numbered nodes and ranges, these are expanded to yield a list of
node names.  For example, `c[1-3,5]-[2-4].fox` yields `c1-2.fox`, `c1-3.fox`, `c1-4.fox`,
`c2-2.fox`, `c2-3.fox`, `c2-4.fox`, `c3-2.fox`, `c3-3.fox`, `c3-4.fox`, `c5-2.fox`, `c5-3.fox`,
`c5-4.fox`.  In a valid range, the first number is no greater than the second number, and
numbers are not repeated.  (The motivation for this feature is that some clusters have very many
nodes and that they group well this way.)

