import ast
import contextlib
import hashlib
import importlib.metadata
import json
import pathlib
import string
import sys
from collections import Counter

import numpy as np
import regex
from nltk.stem import PorterStemmer

SOURCE_SHA256 = "8e3be5d57ff2ff9ec5cd05939592f468c5f3f1fd95d13e431932bdf6bf0fd6fd"
FUNCTIONS = {"normalize_answer", "f1_score", "f1", "eval_question_answering"}


def main():
    for package, expected in (("nltk", "3.8.1"), ("numpy", "1.26.0"), ("regex", "2022.10.31")):
        if importlib.metadata.version(package) != expected:
            raise RuntimeError(f"LoCoMo scorer requires {package}=={expected}")
    source_path = pathlib.Path(__file__).parent / "upstream" / "evaluation.py"
    source = source_path.read_bytes()
    if hashlib.sha256(source).hexdigest() != SOURCE_SHA256:
        raise RuntimeError("Pinned LoCoMo scorer digest mismatch")
    original = ast.parse(source, filename=str(source_path))
    functions = [node for node in original.body if isinstance(node, ast.FunctionDef) and node.name in FUNCTIONS]
    if {node.name for node in functions} != FUNCTIONS:
        raise RuntimeError("Pinned LoCoMo scoring functions missing")
    namespace = {"regex": regex, "string": string, "np": np, "Counter": Counter, "ps": PorterStemmer()}
    exec(compile(ast.Module(body=functions, type_ignores=[]), str(source_path), "exec"), namespace)
    rows = json.load(sys.stdin)
    if not isinstance(rows, list) or not rows:
        raise ValueError("Expected nonempty scoring rows")
    with contextlib.redirect_stdout(sys.stderr):
        scores, _, _ = namespace["eval_question_answering"](rows)
    json.dump({"scores": [float(score) for score in scores], "source_sha256": SOURCE_SHA256}, sys.stdout)


if __name__ == "__main__":
    main()
