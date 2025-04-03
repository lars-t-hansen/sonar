# Sonar JSON format output specification

## Introduction


The top-level object for each output data type is the "...Envelope" object.  Within each
envelope, `Data` and `Errors` are exclusive of each other.

MetadataObject and ErrorObject are shared between the various data types, everything else is
specific to the data type and has a name that clearly indicates that.

Some fields present in the older Sonar data are no longer here, having been deemed redundant or
obsolete.  Some are here in a different form.  Therefore, while old and new data are broadly
compatible, there may be some minor problems translating between them.

## Types

### Type: `NonemptyString`

String value where an empty value is an error, not simply absence of data

### Type: `NonzeroUint`

Uint64 value where zero is an error, not simply absence of datga

### Type: `Timestamp`

RFC3999 localtime+TZO with no sub-second precision: yyyy-mm-ddThh:mm:ss+hh:mm, "Z" for +00:00.

### Type: `Hostname`

Dotted host name or prefix of same, with standard restrictions on character set.

### Type: `Xint`

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

NOTE: These are extracted from the node periodically, currently with Sonar we extract
information every 24h and on node boot.

NOTE: As in every envelope object, the `data` and `errors` are mutually exclusive.

NOTE: In the Go code, the JSON representation can be read with ConsumeJSONSysinfo().

#### **`meta`** MetadataObject

Information about the producer and data format

#### **`data`** *SysinfoData

System data, for successful probes

#### **`errors`** []ErrorObject

Error information, for unsuccessful probes

### Type: `SysinfoData`

System data, for successful sysinfo probes

#### **`type`** DataType

Data tag: The value "sysinfo"

#### **`attributes`** SysinfoAttributes

The data themselves

### Type: `SysinfoAttributes`

This object describes a node, its CPUS, devices, topology and software

#### **`time`** Timestamp


#### **`cluster`** Hostname


#### **`node`** Hostname


#### **`os_name`** NonemptyString


#### **`os_release`** NonemptyString


#### **`architecture`** NonemptyString


#### **`sockets`** NonzeroUint


#### **`cores_per_socket`** NonzeroUint


#### **`threads_per_core`** NonzeroUint


#### **`cpu_model`** string


#### **`memory`** NonzeroUint


#### **`topo_svg`** string


#### **`cards`** []SysinfoGpuCard


#### **`software`** []SysinfoSoftwareVersion


### Type: `SysinfoGpuCard`


#### **`index`** uint64


#### **`uuid`** string


#### **`address`** string


#### **`manufacturer`** string


#### **`model`** string


#### **`architecture`** string


#### **`driver`** string


#### **`firmware`** string


#### **`memory`** uint64


#### **`power_limit`** uint64


#### **`max_power_limit`** uint64


#### **`min_power_limit`** uint64


#### **`max_ce_clock`** uint64


#### **`max_memory_clock`** uint64


### Type: `SysinfoSoftwareVersion`


#### **`key`** string


#### **`name`** string


#### **`version`** string


### Type: `SampleEnvelope`


#### **`data`** *SampleData


#### **`errors`** []ErrorObject


#### **`meta`** MetadataObject


### Type: `SampleData`


#### **`type`** DataType


#### **`attributes`** SampleAttributes


### Type: `SampleAttributes`


#### **`time`** Timestamp


#### **`cluster`** Hostname


#### **`node`** Hostname


#### **`system`** SampleSystem


#### **`jobs`** []SampleJob


#### **`errors`** []ErrorObject


### Type: `SampleSystem`


#### **`cpus`** []SampleCpu


#### **`gpus`** []SampleGpu


#### **`used_memory`** uint64


### Type: `SampleCpu`


### Type: `SampleGpu`


#### **`index`** uint64


#### **`uuid`** string


#### **`failing`** uint64


#### **`fan`** uint64


#### **`compute_mode`** string


#### **`performance_state`** Xint


#### **`memory`** uint64


#### **`ce_util`** uint64


#### **`memory_util`** uint64


#### **`temperature`** int64


#### **`power`** uint64


#### **`power_limit`** uint64


#### **`ce_clock`** uint64


#### **`memory_clock`** uint64


### Type: `SampleJob`


#### **`job`** uint64


#### **`user`** string


#### **`epoch`** uint64


#### **`processes`** []SampleProcess


### Type: `SampleProcess`


#### **`resident_memory`** uint64


#### **`virtual_memory`** uint64


#### **`cmd`** string


#### **`pid`** uint64


#### **`ppid`** uint64


#### **`cpu_avg`** float64


#### **`cpu_util`** float64


#### **`cpu_time`** uint64


#### **`rolledup`** int


#### **`gpus`** []SampleProcessGpu


### Type: `SampleProcessGpu`


#### **`uuid`** string


#### **`gpu_util`** float64


#### **`gpu_memory`** uint64


#### **`gpu_memory_util`** float64


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


#### **`time_limit`** Xint


#### **`partition`** string


#### **`reservation`** string


#### **`nodes`** []string


#### **`priority`** Xint


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


