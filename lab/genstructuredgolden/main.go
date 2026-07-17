// Command genstructuredgolden regenerates Go-derived Plantree v1 JSON goldens.
package main

import (
	"bytes"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"os"
	"path/filepath"

	sppb "cloud.google.com/go/spanner/apiv1/spannerpb"
	queryplan "github.com/apstndb/spannerplan"
	"github.com/apstndb/spannerplan/plantree"
)

const contractVersion = 1

type response struct {
	ContractVersion int           `json:"contractVersion"`
	Rows            []plantreeRow `json:"rows"`
}

type plantreeRow struct {
	NodeID           int32               `json:"nodeId"`
	TreePart         string              `json:"treePart"`
	NodeText         string              `json:"nodeText"`
	DisplayName      string              `json:"displayName"`
	Predicates       []string            `json:"predicates"`
	ScalarChildLinks []plantreeChildLink `json:"scalarChildLinks"`
}

type plantreeChildLink struct {
	Type        string `json:"type"`
	Variable    string `json:"variable"`
	Description string `json:"description"`
	DisplayName string `json:"displayName"`
	ChildIndex  int32  `json:"childIndex"`
	IsPredicate bool   `json:"isPredicate"`
}

type fixture struct {
	input  string
	output string
}

func main() {
	repoRoot := flag.String("repo-root", "../..", "path to the spannerplan-rs repository root")
	check := flag.Bool("check", false, "fail when checked-in golden differs from generated output")
	flag.Parse()

	root, err := filepath.Abs(*repoRoot)
	if err != nil {
		fail(err)
	}

	fixtures := []fixture{
		{
			input:  filepath.Join("testdata", "reference", "dca.yaml"),
			output: filepath.Join("testdata", "golden", "dca_plantree_rows_current.json"),
		},
		{
			input:  filepath.Join("testdata", "reference", "distributed_cross_apply.yaml"),
			output: filepath.Join("testdata", "golden", "dcaplan_plantree_rows_current.json"),
		},
	}

	for _, item := range fixtures {
		if err := generateOne(root, item, *check); err != nil {
			fail(err)
		}
	}
}

func generateOne(root string, item fixture, check bool) error {
	inputPath := filepath.Join(root, item.input)
	input, err := os.ReadFile(inputPath)
	if err != nil {
		return fmt.Errorf("read %s: %w", inputPath, err)
	}

	result, err := project(input)
	if err != nil {
		return fmt.Errorf("project %s: %w", inputPath, err)
	}
	encoded, err := json.MarshalIndent(result, "", "  ")
	if err != nil {
		return fmt.Errorf("encode %s: %w", inputPath, err)
	}
	encoded = append(encoded, '\n')

	outputPath := filepath.Join(root, item.output)
	if check {
		current, err := os.ReadFile(outputPath)
		if err != nil {
			return fmt.Errorf("read %s: %w", outputPath, err)
		}
		if !bytes.Equal(current, encoded) {
			return fmt.Errorf("%s is stale; run go run . -repo-root %s", item.output, root)
		}
		return nil
	}

	if err := os.WriteFile(outputPath, encoded, 0o644); err != nil {
		return fmt.Errorf("write %s: %w", outputPath, err)
	}
	return nil
}

func project(input []byte) (response, error) {
	stats, _, err := queryplan.ExtractQueryPlan(input)
	if err != nil {
		return response{}, err
	}
	planNodes := stats.GetQueryPlan().GetPlanNodes()
	qp, err := queryplan.New(planNodes)
	if err != nil {
		return response{}, err
	}

	// Match the Go reference renderer's CURRENT options. This projection does
	// not render table text, but node titles and wrapped tree parts are part of
	// the stable structured contract.
	rows, err := plantree.ProcessPlan(
		qp,
		plantree.WithQueryPlanOptions(
			queryplan.WithKnownFlagFormat(queryplan.KnownFlagFormatLabel),
			queryplan.WithExecutionMethodFormat(queryplan.ExecutionMethodFormatAngle),
			queryplan.WithTargetMetadataFormat(queryplan.TargetMetadataFormatOn),
		),
	)
	if err != nil {
		return response{}, err
	}

	projected := make([]plantreeRow, 0, len(rows))
	for _, row := range rows {
		node := qp.GetNodeByIndex(row.ID)
		if node == nil {
			return response{}, fmt.Errorf("row %d does not resolve to a plan node", row.ID)
		}
		links, err := projectScalarChildLinks(qp, node)
		if err != nil {
			return response{}, err
		}
		projected = append(projected, plantreeRow{
			NodeID:           row.ID,
			TreePart:         row.TreePart,
			NodeText:         row.NodeText,
			DisplayName:      row.DisplayName,
			Predicates:       append([]string{}, row.Predicates...),
			ScalarChildLinks: links,
		})
	}

	return response{ContractVersion: contractVersion, Rows: projected}, nil
}

func projectScalarChildLinks(
	qp *queryplan.QueryPlan,
	node *sppb.PlanNode,
) ([]plantreeChildLink, error) {
	links := []plantreeChildLink{}
	for _, link := range node.GetChildLinks() {
		child := qp.GetNodeByChildLink(link)
		if child == nil {
			return nil, errors.New("child link does not resolve to a plan node")
		}
		if child.GetKind() != sppb.PlanNode_SCALAR {
			continue
		}
		links = append(links, plantreeChildLink{
			Type:        link.GetType(),
			Variable:    link.GetVariable(),
			Description: child.GetShortRepresentation().GetDescription(),
			DisplayName: child.GetDisplayName(),
			ChildIndex:  child.GetIndex(),
			IsPredicate: qp.IsPredicate(link),
		})
	}
	return links, nil
}

func fail(err error) {
	fmt.Fprintln(os.Stderr, err)
	os.Exit(1)
}
