package newfmt

import (
	"fmt"
	"math"
	"regexp"

	"github.com/NordicHPC/sonar/util/formats/oldfmt"
)

// The use case for this is transitional - when we have old data files (or more generally are
// running an older Sonar) and want to store the data in a new database or send old data to the
// Kafka broker.
//
// This is harder than New -> Old.  We don't have: OsName, OsRelease, TopoSVG, Software, Cluster.
// Of those, only OsName, OsRelease and Cluster matter much and can be passed as optional
// parameters.
//
// We also don't quite have Sockets, CoresPerSocket, ThreadsPerCore, CpuModel though we can parse
// those from the Description, which has been the same since time immemorial.

type OldSysinfoAdapter struct {
	OsName string
	OsRelease string
	Cluster string
}

var descMatcher = regexp.MustCompile(`^(\d+)x(\d+)( \(hyperthreaded\))?(.*?), \d+ GiB`)

func OldSysinfoToNew(d *oldfmt.SysinfoEnvelope, adapter OldSysinfoAdapter) (n SysinfoEnvelope) {) {
	n.Meta.Producer = "sonar"
	n.Meta.Version = d.Version
	if d.CpuCores == 0 && d.MemGB == 0 {
		n.Errors = []ErrorObject{
			ErrorObject{
				Time: d.Timestamp,
				Detail: d.Description,
				Cluster: adapter.Cluster,
				Node: d.Hostname,
			},
		}
	} else {
		n.Data = new(SysinfoData)
		n.Data.Type = "sysinfo"
		a := &n.Data.Attributes
		a.Time = d.Timestamp
		a.Cluster = adapter.Cluster
		a.Node = d.Hostname
		a.OsName = adapter.OsName
		a.OsRelease = adapter.OsRelease
		// TODO: Not clear if we can continue if the match fails, we could consider falling back to
		// only sockets, cores per socket, threads and ignoring the model.  But don't know why it
		// would fail.
		if m := descMatcher.FindStringSubmatch(d.Description); m != nil {
			a.Sockets, _ = strconv.ParseUint(m[1], 10, 64)
			a.CoresPerSocket, _ = strconv.ParseUint(m[2], 10, 64)
			threads := 1
			if m[3] != "" {
				threads = 2
			}
			a.ThreadsPerCore = threads
			a.CpuModel = strings.TrimSpace(m[4])
		}
		a.Memory = d.MemGB * 1024 * 1024
		if len(d.GpuInfo) > 0 {
			a.Cards = make([]SysinfoGpuCard, len(d.GpuInfo))
			for i, c := d.GpuInfo {
				a.Cards[i].BusAddress = c.Address
				// TODO: etc
			}
		}
	}
	return
}
