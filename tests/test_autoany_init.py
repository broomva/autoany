"""Tests for the autoany scaffold initializer."""

import json
import os
import shutil
import subprocess
import sys
import tempfile

import pytest

SCRIPT = os.path.join(
    os.path.dirname(os.path.dirname(__file__)), "autoany", "scripts", "autoany_init.py"
)


@pytest.fixture
def tmp_dir():
    d = tempfile.mkdtemp(prefix="autoany_test_")
    yield d
    shutil.rmtree(d, ignore_errors=True)


def run_init(name, domain="generic", path="."):
    result = subprocess.run(
        [sys.executable, SCRIPT, name, "--domain", domain, "--path", path],
        capture_output=True,
        text=True,
    )
    return result


class TestScaffoldCreation:
    def test_generic_domain(self, tmp_dir):
        result = run_init("my-project", "generic", tmp_dir)
        assert result.returncode == 0
        output = json.loads(result.stdout)
        assert output["status"] == "success"
        assert output["project"] == "my-project"
        assert output["domain"] == "generic"

    def test_all_domains(self, tmp_dir):
        for domain in ["code", "rag", "workflow", "etl", "ui", "generic"]:
            name = f"test-{domain}"
            result = run_init(name, domain, tmp_dir)
            assert result.returncode == 0, (
                f"Failed for domain: {domain}\n{result.stderr}"
            )
            output = json.loads(result.stdout)
            assert output["status"] == "success"

    def test_creates_expected_files(self, tmp_dir):
        run_init("test-proj", "code", tmp_dir)
        project_dir = os.path.join(tmp_dir, "test-proj")
        assert os.path.isfile(os.path.join(project_dir, "problem-spec.yaml"))
        assert os.path.isfile(os.path.join(project_dir, "eval", "run_eval.sh"))
        assert os.path.isfile(os.path.join(project_dir, "ledger.schema.json"))
        assert os.path.isfile(os.path.join(project_dir, "README.md"))
        assert os.path.isfile(os.path.join(project_dir, "ledger.jsonl"))
        assert os.path.isdir(os.path.join(project_dir, "artifacts"))
        assert os.path.isdir(os.path.join(project_dir, "harness"))

    def test_eval_script_is_executable(self, tmp_dir):
        run_init("test-proj", "code", tmp_dir)
        eval_path = os.path.join(tmp_dir, "test-proj", "eval", "run_eval.sh")
        assert os.access(eval_path, os.X_OK)

    def test_problem_spec_contains_domain(self, tmp_dir):
        run_init("rag-test", "rag", tmp_dir)
        spec_path = os.path.join(tmp_dir, "rag-test", "problem-spec.yaml")
        with open(spec_path) as f:
            content = f.read()
        assert "rag" in content
        assert "maximize" in content or "answer_accuracy" in content

    def test_duplicate_project_fails(self, tmp_dir):
        run_init("dup-proj", "generic", tmp_dir)
        result = run_init("dup-proj", "generic", tmp_dir)
        assert result.returncode != 0

    def test_help_flag(self):
        result = subprocess.run(
            [sys.executable, SCRIPT, "--help"],
            capture_output=True,
            text=True,
        )
        assert result.returncode == 0
        assert "Bootstrap" in result.stdout

    def test_json_output_structure(self, tmp_dir):
        result = run_init("json-test", "code", tmp_dir)
        output = json.loads(result.stdout)
        assert "status" in output
        assert "project" in output
        assert "domain" in output
        assert "path" in output
        assert "files_created" in output
        assert "next_steps" in output
        assert isinstance(output["files_created"], list)
        assert isinstance(output["next_steps"], list)


class TestTicketClassifierExample:
    """Validate the example project runs correctly."""

    @pytest.fixture
    def example_dir(self):
        return os.path.join(
            os.path.dirname(os.path.dirname(__file__)),
            "examples",
            "ticket-classifier",
        )

    def test_evaluator_runs(self, example_dir):
        evaluator = os.path.join(example_dir, "eval", "evaluate.py")
        classifier = os.path.join(example_dir, "artifacts", "classifier.py")
        result = subprocess.run(
            [sys.executable, evaluator, classifier],
            capture_output=True,
            text=True,
        )
        assert result.returncode == 0
        output = json.loads(result.stdout)
        assert "score" in output
        assert "correct" in output
        assert "total" in output
        assert output["total"] == 30
        assert 0 <= output["score"] <= 1.0

    def test_eval_set_valid_jsonl(self, example_dir):
        eval_set = os.path.join(example_dir, "eval", "eval_set.jsonl")
        with open(eval_set) as f:
            lines = [line for line in f if line.strip()]
        assert len(lines) == 30
        for line in lines:
            item = json.loads(line)
            assert "text" in item
            assert "label" in item
            assert item["label"] in ("billing", "account", "bug")

    def test_classifier_returns_valid_labels(self, example_dir):
        # Import the classifier module
        import importlib.util

        classifier_path = os.path.join(example_dir, "artifacts", "classifier.py")
        spec = importlib.util.spec_from_file_location("classifier", classifier_path)
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)

        valid_labels = {"billing", "account", "bug"}
        test_inputs = [
            "I need a refund",
            "Can't log in",
            "App crashes on upload",
            "random text here",
        ]
        for text in test_inputs:
            label = mod.classify(text)
            assert label in valid_labels, f"Invalid label '{label}' for '{text}'"
