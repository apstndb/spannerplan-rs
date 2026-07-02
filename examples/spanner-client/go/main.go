package main

/*
#cgo CFLAGS: -I${SRCDIR}/../../../crates/spannerplan-ffi
#cgo darwin LDFLAGS: ${SRCDIR}/../../../target/debug/libspannerplan_ffi.dylib
#cgo linux LDFLAGS: -L${SRCDIR}/../../../target/debug -lspannerplan_ffi
#include "spannerplan.h"
#include <stdlib.h>
*/
import "C"

import (
	"context"
	"flag"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"unsafe"

	"cloud.google.com/go/spanner"
	"google.golang.org/api/iterator"
	spannerpb "google.golang.org/genproto/googleapis/spanner/v1"
	"google.golang.org/protobuf/proto"
)

type cliOptions struct {
	queryMode string
	project   string
	instance  string
	database  string
	sql       string
}

func envOrEmpty(name string) string {
	return strings.TrimSpace(os.Getenv(name))
}

func loadSQL(query, queryFile string) (string, error) {
	if strings.TrimSpace(query) != "" {
		return strings.TrimSpace(query), nil
	}
	path := queryFile
	if path == "" {
		path = filepath.Join("..", "query.sql")
	}
	data, err := os.ReadFile(path)
	if err != nil {
		return "", fmt.Errorf("read query file %q: %w", path, err)
	}
	return strings.TrimSpace(string(data)), nil
}

func parseCliOptions() (cliOptions, error) {
	defaultMode := strings.ToUpper(envOrEmpty("SPANNER_QUERY_MODE"))
	if defaultMode == "" {
		defaultMode = "PLAN"
	}

	queryMode := flag.String("query-mode", defaultMode, "Spanner execute-sql mode: PLAN or PROFILE")
	project := flag.String("project", envOrEmpty("SPANNER_PROJECT_ID"), "GCP project id")
	instance := flag.String("instance", envOrEmpty("SPANNER_INSTANCE_ID"), "Spanner instance id")
	database := flag.String("database", envOrEmpty("SPANNER_DATABASE_ID"), "Spanner database id")
	query := flag.String("query", envOrEmpty("SPANNER_QUERY"), "SQL text (overrides --query-file)")
	queryFile := flag.String("query-file", envOrEmpty("SPANNER_QUERY_FILE"), "SQL file (default: ../query.sql)")
	flag.Parse()

	mode := strings.ToUpper(*queryMode)
	if mode != "PLAN" && mode != "PROFILE" {
		return cliOptions{}, fmt.Errorf("query mode must be PLAN or PROFILE, got: %s", mode)
	}
	if strings.TrimSpace(*project) == "" {
		return cliOptions{}, fmt.Errorf("missing required value: set --project or SPANNER_PROJECT_ID")
	}
	if strings.TrimSpace(*instance) == "" {
		return cliOptions{}, fmt.Errorf("missing required value: set --instance or SPANNER_INSTANCE_ID")
	}
	if strings.TrimSpace(*database) == "" {
		return cliOptions{}, fmt.Errorf("missing required value: set --database or SPANNER_DATABASE_ID")
	}

	sql, err := loadSQL(*query, *queryFile)
	if err != nil {
		return cliOptions{}, err
	}

	return cliOptions{
		queryMode: mode,
		project:   strings.TrimSpace(*project),
		instance:  strings.TrimSpace(*instance),
		database:  strings.TrimSpace(*database),
		sql:       sql,
	}, nil
}

func renderModeFor(queryMode string) string {
	if queryMode == "PROFILE" {
		return "PROFILE"
	}
	return "PLAN"
}

func renderWire(planWire []byte, renderMode string) (string, error) {
	if len(planWire) == 0 {
		return "", fmt.Errorf("empty plan wire bytes")
	}

	mode := C.CString(renderMode)
	format := C.CString("CURRENT")
	defer C.free(unsafe.Pointer(mode))
	defer C.free(unsafe.Pointer(format))

	var isError C.int
	ptr := (*C.uchar)(unsafe.Pointer(&planWire[0]))
	out := C.spannerplan_render_tree_table_wire(
		ptr,
		C.size_t(len(planWire)),
		mode,
		format,
		nil,
		&isError,
	)
	if out == nil {
		return "", fmt.Errorf("native render returned NULL")
	}
	defer C.spannerplan_string_free(out)

	text := C.GoString(out)
	if isError != 0 {
		return "", fmt.Errorf("%s", text)
	}
	return text, nil
}

func fetchQueryPlan(ctx context.Context, client *spanner.Client, sql, queryMode string) (*spannerpb.QueryPlan, error) {
	stmt := spanner.Statement{SQL: sql}
	tx := client.Single()

	if queryMode == "PLAN" {
		return tx.AnalyzeQuery(ctx, stmt)
	}

	mode := spannerpb.ExecuteSqlRequest_PROFILE
	iter := tx.QueryWithOptions(ctx, stmt, spanner.QueryOptions{Mode: &mode})
	defer iter.Stop()
	for {
		_, err := iter.Next()
		if err == iterator.Done {
			break
		}
		if err != nil {
			return nil, err
		}
	}
	if iter.QueryPlan == nil {
		return nil, fmt.Errorf("QueryPlan missing from PROFILE query")
	}
	return iter.QueryPlan, nil
}

func main() {
	opts, err := parseCliOptions()
	if err != nil {
		fmt.Fprintf(os.Stderr, "%v\n", err)
		os.Exit(2)
	}

	ctx := context.Background()
	dbPath := fmt.Sprintf("projects/%s/instances/%s/databases/%s", opts.project, opts.instance, opts.database)
	client, err := spanner.NewClient(ctx, dbPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "spanner client: %v\n", err)
		os.Exit(1)
	}
	defer client.Close()

	plan, err := fetchQueryPlan(ctx, client, opts.sql, opts.queryMode)
	if err != nil {
		fmt.Fprintf(os.Stderr, "fetch query plan: %v\n", err)
		os.Exit(1)
	}

	wire, err := proto.Marshal(plan)
	if err != nil {
		fmt.Fprintf(os.Stderr, "marshal QueryPlan: %v\n", err)
		os.Exit(1)
	}

	table, err := renderWire(wire, renderModeFor(opts.queryMode))
	if err != nil {
		fmt.Fprintf(os.Stderr, "render: %v\n", err)
		os.Exit(1)
	}
	io.WriteString(os.Stdout, table)
}
