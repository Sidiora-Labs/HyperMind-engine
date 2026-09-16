"""Generate checked-in Python wire types from the canonical schemas."""
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parents[3]
package = root / "sdk/python/hypermind"
initializer = package / "__init__.py"
prior_initializer = initializer.read_bytes() if initializer.exists() else None
subprocess.run(["flatc", "--python", "--gen-object-api", "-o", str(package.parent), str(root / "schemas/protocol.fbs")], check=True)
if prior_initializer is not None:
    initializer.write_bytes(prior_initializer)
subprocess.run([sys.executable, "-m", "grpc_tools.protoc", f"-I{root / 'schemas'}", f"--python_out={package}", f"--grpc_python_out={package}", str(root / "schemas/hypermind.proto")], check=True)
generated = package / "hypermind_pb2_grpc.py"
generated.write_text(generated.read_text().replace("import hypermind_pb2 as", "from . import hypermind_pb2 as"))
errors = (root / "crates/hm-core/src/error.rs").read_text()
import re
names = re.findall(r"^\s+(\w+) = (\d+),$", errors, re.MULTILINE)
(package / "_errors.py").write_text("# Generated from hm-core/src/error.rs.\nCODES = " + repr({int(number): "k" + name for name, number in names}) + "\n")
