import contextlib
import io
from pathlib import Path

import pytest

from spannerplan.cli import USAGE_EXIT, run

ROOT = Path(__file__).resolve().parents[3]
FIXTURE = ROOT / "testdata" / "reference" / "dca.yaml"


def test_help_exits_zero():
    stderr = io.StringIO()
    with contextlib.redirect_stderr(stderr):
        code = run(["-h"], b"")
    assert code == 0
    assert "-mode" in stderr.getvalue()
    assert stderr.getvalue().startswith("Usage of rendertree:")


def test_unknown_flag_exits_two():
    stderr = io.StringIO()
    with contextlib.redirect_stderr(stderr):
        code = run(["-unknown"], b"")
    err = stderr.getvalue()
    assert code == USAGE_EXIT
    assert "flag provided but not defined" in err
    assert "Usage of rendertree:" in err


def test_render_fixture(capsys):
    code = run(["-mode", "plan"], FIXTURE.read_bytes())
    captured = capsys.readouterr()
    assert code == 0
    assert "Distributed Cross Apply" in captured.out
    assert captured.err == ""


def test_invalid_mode_exits_two():
    stderr = io.StringIO()
    with contextlib.redirect_stderr(stderr):
        code = run(["-mode", "bogus"], FIXTURE.read_bytes())
    assert code == USAGE_EXIT
    assert "Invalid value for -mode flag" in stderr.getvalue()
