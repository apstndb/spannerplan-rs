from pathlib import Path

import pytest

from spannerplan import RenderError, render_tree_table_json

ROOT = Path(__file__).resolve().parents[3]
FIXTURE = ROOT / "testdata" / "reference" / "dca.yaml"
GOLDEN = ROOT / "testdata" / "golden" / "dca_plan_current.txt"


def test_render_fixture():
  output = render_tree_table_json(FIXTURE.read_text())
  assert "Distributed Cross Apply" in output


def test_render_golden_matches_dca_plan_current():
  output = render_tree_table_json(
    FIXTURE.read_text(), mode="PLAN", format="CURRENT"
  )
  assert output == GOLDEN.read_text()


def test_render_invalid_json():
  with pytest.raises(RenderError):
    render_tree_table_json("not json")
