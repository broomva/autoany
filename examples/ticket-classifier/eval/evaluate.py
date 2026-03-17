#!/usr/bin/env python3
"""
IMMUTABLE evaluator for the ticket classifier EGRI loop.
DO NOT MODIFY during trials.

Loads eval_set.jsonl, runs the classifier on each ticket,
computes accuracy, and outputs structured JSON to stdout.
"""

import importlib.util
import json
import os
import sys
import time

EVAL_SET = os.path.join(os.path.dirname(__file__), "eval_set.jsonl")


def load_eval_set():
    with open(EVAL_SET) as f:
        return [json.loads(line) for line in f if line.strip()]


def load_classifier(path):
    spec = importlib.util.spec_from_file_location("classifier", path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod.classify


def main():
    classifier_path = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
        os.path.dirname(__file__), "..", "artifacts", "classifier.py"
    )

    eval_set = load_eval_set()
    classify = load_classifier(classifier_path)

    correct = 0
    total = len(eval_set)
    errors = []

    start = time.time()
    for item in eval_set:
        try:
            prediction = classify(item["text"])
            if prediction == item["label"]:
                correct += 1
            else:
                errors.append({
                    "text": item["text"],
                    "expected": item["label"],
                    "predicted": prediction,
                })
        except Exception as e:
            errors.append({
                "text": item["text"],
                "expected": item["label"],
                "predicted": None,
                "error": str(e),
            })
    duration = time.time() - start

    accuracy = correct / total if total > 0 else 0.0

    result = {
        "score": accuracy,
        "correct": correct,
        "total": total,
        "duration_s": round(duration, 4),
        "constraints_passed": duration < 1.0,
        "constraint_violations": [] if duration < 1.0 else ["runtime_exceeded"],
        "errors": errors,
    }

    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
