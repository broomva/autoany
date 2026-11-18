"""Keep generated stubs and ledger schema aligned with the runtime evidence gate."""

import json
import subprocess
import sys
from pathlib import Path

import jsonschema
import pytest

ROOT = Path(__file__).parents[1]
SCHEMA = json.loads((ROOT / "autoany/assets/ledger.schema.json").read_text())


def record(
    control="pass",
    target="not_detected",
    falsifies=True,
    verdict="refuted",
    action="discarded",
):
    return {
        "trial_id": "test",
        "timestamp": "2026-11-18T00:00:00Z",
        "parent_state": "base",
        "mutation": {"operator": "test", "description": "controlled observation"},
        "outcome": {
            "score": 100,
            "constraints_passed": True,
            "evidence": {
                "claim": "target produces signal",
                "conditions": "run-1, threshold 0.1",
                "observation_ref": "raw#target",
                "positive_control": control,
                "control_ref": "raw#control" if control == "pass" else None,
                "target_outcome": target,
                "falsification_criterion": "controlled null",
                "absence_refutes": falsifies,
                "claim_verdict": verdict,
            },
        },
        "decision": {"action": action},
    }


def test_legacy_and_controlled_records_validate():
    r = record()
    jsonschema.validate(r, SCHEMA)
    del r["outcome"]["evidence"]
    jsonschema.validate(r, SCHEMA)


@pytest.mark.parametrize("control", ["fail", "missing", "error", "skipped"])
def test_unverified_controls_cannot_serialize_refutation(control):
    with pytest.raises(jsonschema.ValidationError):
        jsonschema.validate(record(control=control), SCHEMA)
    jsonschema.validate(
        record(
            control=control,
            verdict="inconclusive_no_positive_control",
            action="escalated",
        ),
        SCHEMA,
    )


@pytest.mark.parametrize(
    "target,flag,verdict",
    [
        ("not_detected", False, "not_detected"),
        ("error", True, "inconclusive"),
        ("skipped", True, "inconclusive"),
    ],
)
def test_inconclusive_ledger_cannot_claim_promotion(target, flag, verdict):
    r = record(target=target, falsifies=flag, verdict=verdict)
    jsonschema.validate(r, SCHEMA)
    r["decision"]["action"] = "promoted"
    with pytest.raises(jsonschema.ValidationError):
        jsonschema.validate(r, SCHEMA)


def test_passing_control_needs_nonempty_reference():
    r = record()
    for value in (None, "", "   "):
        r["outcome"]["evidence"]["control_ref"] = value
        with pytest.raises(jsonschema.ValidationError):
            jsonschema.validate(r, SCHEMA)


def test_scaffold_stub_is_explicitly_unimplemented_and_cannot_pass(tmp_path):
    result = subprocess.run(
        [
            sys.executable,
            str(ROOT / "autoany/scripts/autoany_init.py"),
            "control-test",
            "--path",
            str(tmp_path),
        ],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0
    evaluator = tmp_path / "control-test/eval/run_eval.sh"
    result = subprocess.run(["bash", str(evaluator)], capture_output=True, text=True)
    assert result.returncode == 0
    output = json.loads(result.stdout)
    assert not output["constraints_passed"]
    assert output["evidence"]["claim_verdict"] == "inconclusive_no_positive_control"
    schema = json.loads((tmp_path / "control-test/ledger.schema.json").read_text())
    r = record(action="escalated")
    r["outcome"] = output
    jsonschema.validate(r, schema)
    r["decision"]["action"] = "promoted"
    with pytest.raises(jsonschema.ValidationError):
        jsonschema.validate(r, schema)


def test_actual_invalid_evaluator_trials_conform_to_canonical_schema():
    result = subprocess.run(
        [
            "cargo",
            "test",
            "--manifest-path",
            str(ROOT / "autoany-core/Cargo.toml"),
            "--test",
            "evidence_test",
            "inconsistent_reports_produce_schema_safe_ledger_records",
            "--",
            "--nocapture",
        ],
        capture_output=True,
        text=True,
        check=True,
    )
    records = [
        json.loads(line.split("LEDGER_RECORD:", 1)[1])
        for line in result.stdout.splitlines()
        if line.startswith("LEDGER_RECORD:")
    ]
    assert len(records) == 3
    for trial in records:
        jsonschema.validate(trial, SCHEMA)
        assert trial["decision"]["action"] == "escalated"
        assert trial["outcome"]["evaluator_metadata"]["rejected_evidence"]
