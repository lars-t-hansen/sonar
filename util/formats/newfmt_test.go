// The purpose of this test:
//
// - touch every defined data field at least once and make sure it has the expected value
// - check some internal consistency, eg Errors xor Data
// - check the TRES parser
// - (less important) check the JSON consumer logic

package formats

import (
	"os"
	"strings"
	"testing"

	"github.com/NordicHPC/sonar/util/formats/newfmt"
)

func TestNewJSONSysinfo(t *testing.T) {
	f, err := os.Open("testdata/newfmt_sysinfo.json")
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	// There are three records: one for ml1 with GPUs, one for c1-6.fox without, one error
	var iter int
	err = newfmt.ConsumeJSONSysinfo(f, false, func(info *newfmt.SysinfoEnvelope) {
		switch iter {
		case 0:
			assert(t, info.Meta.Producer == "sonar", "#0 producer")
			assert(t, info.Meta.Version == "0.13.0", "#0 version")
			assert(t, info.Errors == nil, "#0 errors")
			assert(t, info.Data.Type == "sysinfo", "#0 type")
			a := info.Data.Attributes
			assert(t, a.Time == "2025-03-01T00:00:01+01:00", "#0 time")
			assert(t, a.Cluster == "mlx.hpc.uio.no", "#0 cluster")
			assert(t, a.Node == "ml1.hpc.uio.no", "#0 node")
			assert(t, a.Os == "Linux", "#0 os")
			assert(t, a.OsVersion == "4.18.0-553.30.1.el8_10.x86_64", "#0 os-version")
			assert(t, len(a.Cores) == 8, "#0 cores")
			assert(t, a.Cores[1].Index == 1, "#0 core index")
			assert(t, a.Cores[1].Model == "yoyodyne-3", "#0 core model")
			assert(t, a.Memory == 131072000, "#0 memory")
			assert(t, strings.HasPrefix(a.Description, "2x14 (hyperthreaded) Intel(R) Xeon(R)"), "#0 desc")
			assert(t, len(a.Cards) == 3, "#0 cards")
			c := a.Cards[1]
			assert(t, c.Index == 1, "#0 card index")
			assert(t, c.UUID == "GPU-be013a01-364d-ca23-f871-206fe3f259ba", "#0 card UUID")
			assert(t, c.Address == "00000000:3B:00.0", "#0 card address")
			assert(t, c.Manufacturer == "NVIDIA", "#0 card manufacturer")
			assert(t, c.Model == "NVIDIA GeForce RTX 2080 Ti", "#0 card model")
			assert(t, c.Architecture == "Turing", "#0 card arch")
			assert(t, c.Driver == "550.127.08", "#0 card driver")
			assert(t, c.Firmware == "12.4", "#0 card firmware")
			assert(t, c.Memory == 11534336, "#0 card memory")
			assert(t, c.PowerLimit == 250, "#0 card power limit")
			assert(t, c.MaxPowerLimit == 280, "#0 card max power limit")
			assert(t, c.MinPowerLimit == 100, "#0 card min power limit")
			assert(t, c.MaxCEClock == 2100, "#0 card max ce clock")
			assert(t, c.MaxMemClock == 7000, "#0 card max memory clock")
			assert(t, len(a.Software) == 0, "#0 software")
		case 1:
			a := info.Data.Attributes
			assert(t, a.Cluster == "fox.educloud.no", "#1 cluster")
			assert(t, a.Node == "c1-6.fox", "#1 node")
		case 2:
			assert(t, info.Errors != nil, "#2 errors")
			assert(t, info.Errors[0].Detail == "Node not cooperating", "#2 msg")
		}
		iter++
	})
	if err != nil {
		t.Fatal(err)
	}
	assert(t, iter == 3, "Iteration count")
}

// Test that unknown fields are caught in strict mode

func TestNewJSONSysinfo2(t *testing.T) {
	f := strings.NewReader(`{"zappa":"hello"}`)
	err := newfmt.ConsumeJSONSysinfo(f, true, func(info *newfmt.SysinfoEnvelope) {})
	assert(t, err != nil && strings.Index(err.Error(), "unknown field") != -1, "Unknown field #1 msg")

	f = strings.NewReader(`{"meta":{"zappa":"hello"}}`)
	err = newfmt.ConsumeJSONSysinfo(f, true, func(info *newfmt.SysinfoEnvelope) {})
	assert(t, err != nil && strings.Index(err.Error(), "unknown field") != -1, "Unknown field #2 msg")
}

func TestNewJSONSysinfoActual(t *testing.T) {
	// Here I want to:
	// - run `sonar sysinfo` once
	// - parse the output in strict mode
	// This ensures that:
	// - all emitted, known fields have the right types
	// - no unknown fields are emitted
	// Do this on enough machines and we'll have a decent test of whether Sonar works in practice.
}

// TODO: Samples

func TestNewJSONSlurmJobs(t *testing.T) {
	f, err := os.Open("testdata/newfmt_slurmjobs.json")
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	// There are four records in the file: good, good, error, good.
	var iter int
	err = newfmt.ConsumeJSONJobs(f, false, func(info *newfmt.JobsEnvelope) {
		switch iter {
		case 0:
			assert(t, info.Errors == nil, "#0 defined")
			assert(t, info.Meta.Producer == "sonar", "#0 producer")
			assert(t, info.Meta.Version == "0.13.0", "#0 version")
			assert(t, info.Data.Type == "jobs", "#0 type")
			a := info.Data.Attributes
			assert(t, a.Cluster == "fox.educloud.no", "#0 cluster")
			assert(t, a.Time == "2025-03-11T09:31:00+01:00", "#0 time")
			assert(t, len(a.SlurmJobs) == 5, "#0 len")
			j := a.SlurmJobs[1]
			assert(t, j.JobID == "1382657.batch", "#0 id")
			tres, dropped := newfmt.DecodeSlurmTRES(j.AllocTRES)
			assert(t, len(dropped) == 0, "#0 tres dropped")
			assert(t, len(tres) == 3, "#0 tres len")
			assert(t, tres[1].Key == "mem", "#0 tres key")
			assert(t, tres[1].Value == 472.50 * 1024 * 1024 * 1024, "#0 tres val")
		case 2:
			assert(t, info.Errors != nil, "#2 error")
			e := info.Errors[0]
			assert(t, e.Detail == "No can do", "#2 msg")
			assert(t, e.Time == "2025-03-11T09:31:00+01:00", "#2 time")
		}
		iter++
	})
	if err != nil {
		t.Fatal(err)
	}
	assert(t, iter == 4, "Iteration count")
}

func TestNewJSONSlurmJobs2(t *testing.T) {
	f := strings.NewReader(`{"zappa":"hello"}`)
	err := newfmt.ConsumeJSONJobs(f, true, func(info *newfmt.JobsEnvelope) {})
	assert(t, err != nil && strings.Index(err.Error(), "unknown field") != -1, "Unknown field #1 msg")

	f = strings.NewReader(`{"meta":{"zappa":"hello"}}`)
	err = newfmt.ConsumeJSONJobs(f, true, func(info *newfmt.JobsEnvelope) {})
	assert(t, err != nil && strings.Index(err.Error(), "unknown field") != -1, "Unknown field #2 msg")
}

func TestDecodeSlurmTRES(t *testing.T) {
	xs, ys := newfmt.DecodeSlurmTRES("billing=20,cpu=20,gres/gpu:rtx30=1,gres/gpu=1,mem=50G,zappa,node=1")
	assert(t, len(xs) == 6, "#0 kv len")
	assert(t, len(ys) == 1, "#0 dropped len")
	assert(t, ys[0] == "zappa", "#0 dropped")
	keys := []string{"billing","cpu","gres/gpu:rtx30","gres/gpu","mem","node"}
	values := []any{int64(20), int64(20), int64(1), int64(1), int64(50*1024*1024*1024), int64(1)}
	for i := range keys {
		assert(t, xs[i].Key == keys[i], "#0 key")
		assert(t, xs[i].Value == values[i], "#0 value")
	}
}
