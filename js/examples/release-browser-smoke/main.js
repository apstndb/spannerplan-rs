import { parse } from "yaml";
import { internalPlantreeRowsV1Alpha2 } from "@spannerplan/core/browser";

const output = document.querySelector("#release-smoke-result");

let result;
try {
  const source = await fetch("/dca.yaml").then((response) => {
    if (!response.ok) throw new Error(`fixture request failed: ${response.status}`);
    return response.text();
  });
  const plan = parse(source);
  const response = await internalPlantreeRowsV1Alpha2(plan);
  if ("error" in response) throw new Error(response.error);
  const predicateLinks = response.rows.flatMap((row) =>
    row.scalarChildLinks.filter((link) => link.isPredicate),
  );
  const expectedRootNodeText = "Distributed Union on AlbumsByAlbumTitle <Row>";
  if (response.contractVersion !== 2) throw new Error("unexpected contract version");
  if (response.rows.length === 0) throw new Error("Plantree rows are empty");
  if (response.rows[0].nodeId !== 0) throw new Error("Plantree root nodeId is not 0");
  if (response.rows[0].rowId !== "0" || response.rows[0].parentRowId !== null) {
    throw new Error("Plantree root occurrence identity is invalid");
  }
  if (response.rows[0].nodeText !== expectedRootNodeText) {
    throw new Error(`unexpected Plantree root nodeText: ${response.rows[0].nodeText}`);
  }
  if (!predicateLinks.some((link) => link.displayName === "Function")) {
    throw new Error("predicate scalar-link evidence is absent");
  }
  result = {
    status: "ok",
    contractVersion: response.contractVersion,
    rowCount: response.rows.length,
    rootNodeId: response.rows[0].nodeId,
    rootNodeText: response.rows[0].nodeText,
    predicateLinks: predicateLinks.length,
  };
} catch (error) {
  result = {
    status: "error",
    error: error instanceof Error ? error.message : String(error),
  };
}

output.dataset.status = result.status;
output.textContent = JSON.stringify(result);
const reportResponse = await fetch("/__release-smoke-result", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify(result),
});
if (!reportResponse.ok) {
  throw new Error(`result report failed: ${reportResponse.status}`);
}
