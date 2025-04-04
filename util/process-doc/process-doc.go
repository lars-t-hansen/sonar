// This reads input from stdin and extracts some documentation, and generates various output on
// stdout, depending on options.  A typical input file is ../formats/newfmt/types.go.  Typical
// output is markdown documentation, or field name definitions to be used by Rust code.  Try -h.
//
// The format is this:
//
// Input     ::= Preamble? (TypeDefn | Junk)*
// Preamble  ::= PreFlag Doc*
// PreFlag   ::= <line starting with "///+preamble" after blank stripping
// TypeDefn  ::= Doc+ Blank* Type FieldDefn*
// Doc       ::= <line starting with "///" after blank stripping>
// Blank     ::= <line that's empty after blank stripping>
// Type      ::= <contextually, line that starts with "type" after blank stripping>
// FieldDefn ::= Blank* Doc+ Blank* Field
// Field     ::= <contextually, line that starts with a capitalized identifier and has a json tag>
// Junk      ::= <any other line>
//
// Note that anything that is not a FieldDefn will interrupt the run of fields in a structured type.

package main

import (
	"bufio"
	"flag"
	"fmt"
	"os"
	"regexp"
	"strings"
)

var (
	makeDoc  = flag.Bool("doc", false, "Produce markdown documentation")
	makeRust = flag.Bool("tag", false, "Produce Rust constant JSON field tags")
	warnings = flag.Bool("w", false, "Print warnings")
)

func main() {
	flag.Parse()
	if *makeDoc == *makeRust {
		fmt.Fprintf(os.Stderr, "Must use -doc xor -tag.  Try -h.\n")
		os.Exit(2)
	}
	switch {
	case *makeDoc:
		fmt.Print("# Sonar JSON format output specification\n\n")
	case *makeRust:
		fmt.Print("// AUTOMATICALLY GENERATED.  DO NOT EDIT.\n")
		fmt.Print("#![allow(dead_code)]\n\n") // Should remove this eventually, OK for testing
	}
	lines := make(chan any)
	go producer(lines)
	process(lines)
}

type DocLine struct {
	Lineno int
	Text   string
}

type TypeLine struct {
	Lineno int
	Name   string
}

type FieldLine struct {
	Lineno int
	Name   string
	Type   string
	Json   string
}

type JunkLine struct {
	Lineno int
}

func process(lines <-chan any) {
	doc := make([]string, 0)
	var havePreamble bool
	var currLine any
LineConsumer:
	for {
		currLine = <-lines
	OuterSwitch:
		switch l := currLine.(type) {
		case nil:
			break LineConsumer
		case JunkLine:
			havePreamble, doc = maybePreamble(l.Lineno, havePreamble, doc)
			warnIf(len(doc) > 0, l.Lineno, "Junk following doc")
			doc = doc[0:0]
		case DocLine:
			doc = append(doc, l.Text)
		case TypeLine:
			havePreamble, doc = maybePreamble(l.Lineno, havePreamble, doc)
			warnIf(len(doc) == 0, l.Lineno, "Type without doc")
			emitType(l, doc)
			doc = doc[0:0]
			currType := l.Name
		FieldConsumer:
			for {
				// Important that we reuse the currLine and doc variables
				currLine = <-lines
				switch l := currLine.(type) {
				case nil:
					break LineConsumer
				case JunkLine:
					warnIf(len(doc) > 0, l.Lineno, "Junk following doc")
					doc = doc[0:0]
					break FieldConsumer
				case DocLine:
					doc = append(doc, l.Text)
				case TypeLine:
					goto OuterSwitch
				case FieldLine:
					warnIf(len(doc) == 0, l.Lineno, "Field without doc")
					emitField(l, currType, doc)
					doc = doc[0:0]
				default:
					panic("Should not happen")
				}
			}
		case FieldLine:
			havePreamble, doc = maybePreamble(l.Lineno, havePreamble, doc)
			warnIf(true, l.Lineno, "Field outside of typedecl context")
		default:
			panic("Should not happen")
		}
	}
}

func maybePreamble(l int, havePreamble bool, doc []string) (bool, []string) {
	if len(doc) > 1 && !*makeRust {
		if strings.HasPrefix(doc[0], "+preamble") {
			warnIf(havePreamble, l, "Redundant preamble")
			for _, d := range doc[1:] {
				fmt.Println(d)
			}
			fmt.Println()
			return true, doc[0:0]
		}
	}
	return false, doc
}

func warnIf(cond bool, l int, msg string) {
	if cond && *warnings {
		fmt.Fprintf(os.Stderr, "%d: WARNING: %s\n", l, msg)
	}
}

var printedTypeHeading bool

func maybeTypeHeading() {
	if !printedTypeHeading {
		fmt.Print("## Data types\n\n")
		printedTypeHeading = true
	}
}

func emitType(l TypeLine, doc []string) {
	if *makeDoc {
		maybeTypeHeading()
		fmt.Printf("### Type: `%s`\n\n", l.Name)
		for _, d := range doc {
			fmt.Println(d)
		}
		fmt.Println()
	}
}

func emitField(l FieldLine, currType string, doc []string) {
	switch {
	case *makeDoc:
		fmt.Printf("#### **`%s`** %s\n\n", l.Json, l.Type)
		for _, d := range doc {
			fmt.Println(d)
		}
		fmt.Println()
	case *makeRust:
		// TODO: These are emitted as &str now.  But in a future universe, once all the old
		// formatting code is gone, or maybe even before, they could maybe be of a distinguished
		// type, to prevent literal strings from being used at all.  (It could be an enum wrapping a
		// &str, modulo problems with initialization, or maybe it would be an enum whose value
		// points into some table.)
		//
		// Rust naming conventions: In a given name, the first capital letter X after a lower case
		// letter is transformed to _X.
		//
		// TODO: _ should be inserted between the last two capitals of a run of capitals immediately
		// followed by a lower case letter, so that 'CEClock' becomes '_CE_CLOCK_' no '_CECLOCK_'.
		bs := []byte(currType + l.Name)
		name := ""
		for i := range bs {
			if i > 0 && isUpper(bs[i]) && !isUpper(bs[i-1]) {
				name += "_"
			}
			name += toUpper(bs[i])
		}
		fmt.Printf("pub const %s: &str = \"%s\";\n", name, l.Json)
	}
}

func isUpper(b uint8) bool {
	return b >= 'A' && b <= 'Z'
}

func isLower(b uint8) bool {
	return b >= 'a' && b <= 'z'
}

func toUpper(b uint8) string {
	if isLower(b) {
		return string(b - ('a' - 'A'))
	}
	return string(b)
}

var (
	docRe   = regexp.MustCompile(`^\s*///(.*)$`)
	blankRe = regexp.MustCompile(`^\s*$`)
	typeRe  = regexp.MustCompile(`^\s*type\s+([a-zA-Z0-9_]+)`)
	fieldRe = regexp.MustCompile(`^\s*([A-Z][a-zA-Z0-9_]*)\s+(.*)\s+` + "`" + `json:"(.*)"`)
)

func producer(lines chan<- any) {
	scanner := bufio.NewScanner(os.Stdin)
	var lineno int
	for scanner.Scan() {
		lineno++
		l := scanner.Text()
		if blankRe.MatchString(l) {
			continue
		}
		if m := docRe.FindStringSubmatch(l); m != nil {
			lines <- DocLine{Lineno: lineno, Text: strings.TrimSpace(m[1])}
			continue
		}
		if m := typeRe.FindStringSubmatch(l); m != nil {
			lines <- TypeLine{Lineno: lineno, Name: m[1]}
			continue
		}
		if m := fieldRe.FindStringSubmatch(l); m != nil {
			lines <- FieldLine{Lineno: lineno, Name: m[1], Type: strings.TrimSpace(m[2]), Json: m[3]}
			continue
		}
		lines <- JunkLine{lineno}
	}
	close(lines)
}
