package newfmt

import (
	"regexp"
	"strconv"
	"strings"

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

func toNonemptyString(s string) NonemptyString {
	if s == "" {
		panic("Empty string")
	}
	return NonemptyString(s)
}

func toTimestamp(s string) Timestamp {
	// FIXME
	return Timestamp(s)
}

func toHostname(s string) Hostname {
	// FIXME
	return Hostname(s)
}

func toNonzeroUint(u uint64) NonzeroUint {
	if u == 0 {
		panic("Zero")
	}
	return NonzeroUint(u)
}

func OldSysinfoToNew(d *oldfmt.SysinfoEnvelope, adapter OldSysinfoAdapter) (n SysinfoEnvelope) {
	n.Meta.Producer = "sonar"
	n.Meta.Version = toNonemptyString(d.Version)
	if d.CpuCores == 0 && d.MemGB == 0 {
		n.Errors = []ErrorObject{
			ErrorObject{
				Time: toTimestamp(d.Timestamp),
				Detail: toNonemptyString(d.Description),
				Cluster: toHostname(adapter.Cluster),
				Node: toHostname(d.Hostname),
			},
		}
	} else {
		n.Data = new(SysinfoData)
		n.Data.Type = "sysinfo"
		a := &n.Data.Attributes
		a.Time = toTimestamp(d.Timestamp)
		a.Cluster = toHostname(adapter.Cluster)
		a.Node = toHostname(d.Hostname)
		a.OsName = toNonemptyString(adapter.OsName)
		a.OsRelease = toNonemptyString(adapter.OsRelease)
		// TODO: Not clear if we can continue if the match fails, we could consider falling back to
		// only sockets, cores per socket, threads and ignoring the model.  But don't know why it
		// would fail.
		if m := descMatcher.FindStringSubmatch(d.Description); m != nil {
			n, _ := strconv.ParseUint(m[1], 10, 64)
			a.Sockets = toNonzeroUint(n)
			n, _ = strconv.ParseUint(m[2], 10, 64)
			a.CoresPerSocket = toNonzeroUint(n)
			var threads uint64 = 1
			if m[3] != "" {
				threads = 2
			}
			a.ThreadsPerCore = toNonzeroUint(threads)
			a.CpuModel = strings.TrimSpace(m[4])
		}
		a.Memory = toNonzeroUint(d.MemGB * 1024 * 1024)
		if len(d.GpuInfo) > 0 {
			a.Cards = make([]SysinfoGpuCard, len(d.GpuInfo))
			for i, c := range d.GpuInfo {
				a.Cards[i].Address = c.BusAddress
				// TODO: etc
			}
		}
	}
	return
}
