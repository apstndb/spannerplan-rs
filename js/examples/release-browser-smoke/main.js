import { parse } from "yaml";
import { plantreeRows } from "@spannerplan/core/browser";

const output = document.querySelector("#release-smoke-result");

function report(value, status) {
  output.dataset.status = status;
  output.textContent = JSON.stringify(value);
}

try {
  const source = await fetch("/dca.yaml").then((response) => {
    if (!response.ok) throw new Error(`fixture request failed: ${response.status}`);
    return response.text();
  });
  const plan = parse(source);
  const response = await plantreeRows(plan);
  if ("error" in response) throw new Error(response.error);
  const predicateLinks = response.rows.flatMap((row) =>
    row.scalarChildLinks.filter((link) => link.isPredicate),
  );
  const expectedRootNodeText = "Distributed Union on AlbumsByAlbumTitle <Row>";
  if (response.contractVersion !== 1) throw new Error("unexpected contract version");
  if (response.rows.length === 0) throw new Error("Plantree rows are empty");
  if (response.rows[0].nodeId !== 0) throw new Error("Plantree root nodeId is not 0");
  if (response.rows[0].nodeText !== expectedRootNodeText) {
    throw new Error(`unexpected Plantree root nodeText: ${response.rows[0].nodeText}`);
  }
  if (!predicateLinks.some((link) => link.displayName === "Function")) {
    throw new Error("predicate scalar-link evidence is absent");
  }
  report({
    contractVersion: response.contractVersion,
    rowCount: response.rows.length,
    rootNodeId: response.rows[0].nodeId,
    rootNodeText: response.rows[0].nodeText,
    predicateLinks: predicateLinks.length,
  }, "ok");
} catch (error) {
  report({ error: error instanceof Error ? error.message : String(error) }, "error");
}
