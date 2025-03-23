// New Sonar JSON data format (we no longer support any kind of CSV variant).  MetadataObject and
// ErrorObject are shared between the various data types, everything else is specific to the data
// type and has a name that clearly indicates that.  The top-level object for all data types is the
// "...Envelope" object.  Within the envelopes, `Data` and `Errors` are exclusive of each other.
//
// Some fields present in the older Sonar data are no longer here, having been deemed redundant or
// obsolete.  Some are here in a different form.  Therefore, while old and new data are broadly
// compatible, there may be some minor problems translating between them.
//
// The exact semantics of fields are still defined in the spec in doc/DATA-FORMATS.md.

package newfmt

type MetadataObject struct {
	Producer string `json:"producer"`
	Version  string `json:"version"`
	Format   uint64 `json:"format"`
	// The `Token` is not documented in the spec.  It is a makeshift authorization token (API token)
	// to be used with Envelope.Data.Attributes.Cluster, it proves that the producer of the datum
	// was authorized to produce data for that cluster name.
	Token string `json:"token"`
}

type ErrorObject struct {
	Time    string `json:"time"`
	Detail  string `json:"detail"`
	Cluster string `json:"cluster"`
	Node    string `json:"node"`
}

type KVPair struct {
	Key   string `json:"key"`
	Value string `json:"value"`
}

// JSON representation can be read with ConsumeJSONSysinfo().

type SysinfoEnvelope struct {
	Data   *SysinfoData   `json:"data"`
	Errors []ErrorObject  `json:"errors"`
	Meta   MetadataObject `json:"meta"`
}

type SysinfoData struct {
	Type       string            `json:"type"`
	Attributes SysinfoAttributes `json:"attributes"`
}

type SysinfoAttributes struct {
	Time           string                   `json:"time"`
	Cluster        string                   `json:"cluster"`
	Node           string                   `json:"node"`
	OsName         string                   `json:"os_name"`
	OsRelease      string                   `json:"os_release"`
	Sockets        uint64                   `json:"sockets"`
	CoresPerSocket uint64                   `json:"cores_per_socket"`
	ThreadsPerCore uint64                   `json:"threads_per_core"`
	CpuModel       string                   `json:"cpu_model"`
	Memory         uint64                   `json:"memory"`
	TopoSVG        string                   `json:"topo_svg"` // Base64-encoded SVG
	Cards          []SysinfoGpuCard         `json:"cards"`
	Software       []SysinfoSoftwareVersion `json:"software"`
}

type SysinfoGpuCard struct {
	Index         uint64 `json:"index"`
	UUID          string `json:"uuid"`
	Address       string `json:"address"`
	Manufacturer  string `json:"manufacturer"`
	Model         string `json:"model"`
	Architecture  string `json:"architecture"`
	Driver        string `json:"driver"`
	Firmware      string `json:"firmware"`
	Memory        uint64 `json:"memory"`
	PowerLimit    uint64 `json:"power_limit"`
	MaxPowerLimit uint64 `json:"max_power_limit"`
	MinPowerLimit uint64 `json:"min_power_limit"`
	MaxCEClock    uint64 `json:"max_ce_clock"`
	MaxMemClock   uint64 `json:"max_memory_clock"`
}

type SysinfoSoftwareVersion struct {
	Key     string `json:"key"`
	Name    string `json:"name"`
	Version string `json:"version"`
}

// JSON representation can be read with ConsumeJSONSamples().

type SampleEnvelope struct {
	Data   *SampleData    `json:"data"`
	Errors []ErrorObject  `json:"errors"`
	Meta   MetadataObject `json:"meta"`
}

type SampleData struct {
	Type       string           `json:"type"`
	Attributes SampleAttributes `json:"attributes"`
}

type SampleAttributes struct {
	Time    string        `json:"time"`
	Cluster string        `json:"cluster"`
	Node    string        `json:"node"`
	Attrs   []KVPair      `json:"attrs"`
	System  SampleSystem  `json:"system"`
	Jobs    []SampleJob   `json:"jobs"`
	Errors  []ErrorObject `json:"errors"`
}

type SampleSystem struct {
	Cpus       []SampleCpu `json:"cpus"`
	Gpus       []SampleGpu `json:"gpus"`
	UsedMemory uint64      `json:"used_memory"`
}

type SampleCpu = uint64

type SampleGpu struct {
	Index            uint64 `json:"index"`
	UUID             string `json:"uuid"`
	Failing          uint64 `json:"failing"`
	Fan              uint64 `json:"fan"`
	ComputeMode      string `json:"compute_mode"`
	PerformanceState int64  `json:"performance_state"`
	Memory           uint64 `json:"memory"`
	CEUtil           uint64 `json:"ce_util"`
	MemUtil          uint64 `json:"memory_util"`
	Temperature      int64  `json:"temperature"`
	Power            uint64 `json:"power"`
	PowerLimit       uint64 `json:"power_limit"`
	CEClock          uint64 `json:"ce_clock"`
	MemClock         uint64 `json:"memory_clock"`
}

type SampleJob struct {
	Job       uint64          `json:"job"`
	User      string          `json:"user"`
	Epoch     uint64          `json:"epoch"`
	Processes []SampleProcess `json:"processes"`
}

type SampleProcess struct {
	Resident  uint64             `json:"resident"`
	Virtual   uint64             `json:"virtual"`
	Cmd       string             `json:"cmd"`
	Pid       uint64             `json:"pid"`
	ParentPid uint64             `json:"ppid"`
	CpuAvg    float64            `json:"cpu_avg"`
	CpuUtil   float64            `json:"cpu_util"`
	CpuTime   uint64             `json:"cpu_time"`
	Rolledup  int                `json:"rolledup"`
	Gpus      []SampleProcessGpu `json:"gpus"`
}

type SampleProcessGpu struct {
	UUID       string  `json:"uuid"`
	GpuUtil    float64 `json:"gpu_util"`
	GpuMem     uint64  `json:"gpu_memory"`
	GpuMemUtil float64 `json:"gpu_memory_util"`
}

// JSON representation can be read with ConsumeJSONJobs().

type JobsEnvelope struct {
	Data   *JobsData      `json:"data"`
	Errors []ErrorObject  `json:"errors"`
	Meta   MetadataObject `json:"meta"`
}

type JobsData struct {
	Type       string         `json:"type"`
	Attributes JobsAttributes `json:"attributes"`
}

type JobsAttributes struct {
	Time    string `json:"time"`
	Cluster string `json:"cluster"`
	// There can eventually be other types of jobs, there will be other fields for them here, and
	// the decoder will populate the correct field.  Other fields will be nil.
	SlurmJobs []SlurmJob `json:"slurm_jobs"`
}

type SlurmJob struct {
	JobID        string `json:"JobID"`
	JobIDRaw     string `json:"JobIDRaw"`
	User         string `json:"User"`
	Account      string `json:"Account"`
	State        string `json:"State"`
	Start        string `json:"Start"`
	End          string `json:"End"`
	AveCPU       string `json:"AveCPU"`
	AveDiskRead  string `json:"AveDiskRead"`
	AveDiskWrite string `json:"AveDiskWrite"`
	AveRSS       string `json:"AveRSS"`
	AveVMSize    string `json:"AveVMSize"`
	ElapsedRaw   string `json:"ElapsedRaw"`
	ExitCode     string `json:"ExitCode"`
	Layout       string `json:"Layout"`
	MaxRSS       string `json:"MaxRSS"`
	MaxVMSize    string `json:"MaxVMSize"`
	MinCPU       string `json:"MinCPU"`
	ReqCPUS      string `json:"ReqCPUS"`
	ReqMem       string `json:"ReqMem"`
	ReqNodes     string `json:"ReqNodes"`
	Reservation  string `json:"Reservation"`
	Submit       string `json:"Submit"`
	Suspended    string `json:"Suspended"`
	SystemCPU    string `json:"SystemCPU"`
	TimelimitRaw string `json:"TimelimitRaw"`
	UserCPU      string `json:"UserCPU"`
	NodeList     string `json:"NodeList"`
	Partition    string `json:"Partition"`
	AllocTRES    string `json:"AllocTRES"` // Decode further with DecodeSlurmTRES()
	Priority     string `json:"Priority"`
	JobName      string `json:"JobName"`
}
