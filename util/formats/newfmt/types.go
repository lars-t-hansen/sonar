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
	// The `Token` is not documented in the spec.  It is a makeshift authorization token (API token)
	// to be used with Envelope.Data.Attributes.Cluster, it proves that the producer of the datum
	// was authorized to produce data for that cluster name.
	Token string `json:"token"`
}

type ErrorObject struct {
	Time   string `json:"time"`
	Detail string `json:"detail"`
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
	Time        string                   `json:"time"`
	Cluster     string                   `json:"cluster"`
	Node        string                   `json:"node"`
	Os          string                   `json:"os"`
	OsVersion   string                   `json:"os-version"`
	Cores       []SysinfoCoreModel       `json:"cores"`
	Memory      uint64                   `json:"memory"`
	Description string                   `json:"description"`
	TopoSVG     string                   `json:"topo-svg"` // Base64-encodes SVG
	Cards       []SysinfoGpuCard         `json:"cards"`
	Software    []SysinfoSoftwareVersion `json:"software"`
}

type SysinfoCoreModel struct {
	Index uint64 `json:"index"`
	Model string `json:"model"`
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
	PowerLimit    uint64 `json:"pow-lim"`
	MaxPowerLimit uint64 `json:"max-pow-lim"`
	MinPowerLimit uint64 `json:"min-pow-lim"`
	MaxCEClock    uint64 `json:"max-ce-clock"`
	MaxMemClock   uint64 `json:"max-mem-clock"`
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
	UsedMemory uint64      `json:"used"`
}

type SampleCpu = uint64

type SampleGpu struct {
	Index      uint64 `json:"index"`
	UUID       string `json:"uuid"`
	Bad        bool   `json:"bad"`
	Fan        uint64 `json:"fan"`
	Mode       string `json:"mode"`
	Perf       int64  `json:"perf"`
	Memory     uint64 `json:"memory"`
	CEUtil     uint64 `json:"ce-util"`
	MemUtil    uint64 `json:"mem-util"`
	Temp       int64  `json:"temp"`
	Power      uint64 `json:"pow"`
	PowerLimit uint64 `json:"pow-lim"`
	CEClock    uint64 `json:"ce-clock"`
	MemClock   uint64 `json:"mem-clock"`
}

type SampleJob struct {
	Job       uint64          `json:"job"`
	User      string          `json:"user"`
	Processes []SampleProcess `json:"processes"`
}

type SampleProcess struct {
	Resident  uint64             `json:"resident"`
	Virtual   uint64             `json:"virtual"`
	Cmd       string             `json:"cmd"`
	Pid       uint64             `json:"pid"`
	ParentPid uint64             `json:"ppid"`
	CpuAvg    float64            `json:"cpu-avg"`
	CpuUtil   float64            `json:"cpu-util"`
	CpuTime   uint64             `json:"cpu-time"`
	Rolledup  int                `json:"rolledup"`
	Gpus      []SampleProcessGpu `json:"gpus"`
}

type SampleProcessGpu struct {
	UUID       string  `json:"uuid"`
	GpuUtil    float64 `json:"gpu-util"`
	GpuMem     uint64  `json:"gpu-mem"`
	GpuMemUtil float64 `json:"gpu-mem-util"`
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
	Time      string     `json:"time"`
	Cluster   string     `json:"cluster"`
	// There can be other types of jobs, there will be other fields for them here, and the decoder
	// will populate the correct field.  Other fields will be nil.
	SlurmJobs []SlurmJob `json:"slurm-jobs"`
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
