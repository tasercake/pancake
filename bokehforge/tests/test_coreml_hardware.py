import os
from pathlib import Path
import shutil
import subprocess
import sys

import pytest


@pytest.mark.hardware_accelerator
def test_native_coreml_executes_onnx(tmp_path):
    if sys.platform != "darwin":
        pytest.skip("Core ML requires macOS")

    from bokehforge import _rust
    from bokehforge._attention_setup import _runtime_library

    if not _rust.damage_available("mps"):
        pytest.skip("Requires a local Apple Metal 3 device")
    library = _runtime_library()
    if library is None or not library.is_file():
        pytest.skip("Install the inspection extra for ONNX Runtime")
    onnx = pytest.importorskip("onnx", reason="Install the inspection extra for ONNX fixture generation")
    cargo = shutil.which("cargo")
    if cargo is None:
        pytest.skip("The native Core ML fixture test requires Cargo")

    (tmp_path / "runtime-path.txt").write_text(str(library), encoding="utf-8")
    helper = onnx.helper
    shape = [1, 1, 2, 2]
    graph = helper.make_graph(
        [helper.make_node("Add", ["input", "bias"], ["sum"]),
         helper.make_node("Relu", ["sum"], ["output"])],
        "coreml-smoke",
        [helper.make_tensor_value_info("input", onnx.TensorProto.FLOAT, shape)],
        [helper.make_tensor_value_info("output", onnx.TensorProto.FLOAT, shape)],
        [helper.make_tensor("bias", onnx.TensorProto.FLOAT, shape, [1.0] * 4)],
    )
    model = helper.make_model(graph, opset_imports=[helper.make_opsetid("", 17)], ir_version=9)
    onnx.checker.check_model(model)
    onnx.save(model, tmp_path / "model.onnx")

    result = subprocess.run(
        [cargo, "test", "--release", "--locked", "--lib",
         "attention::tests::configured_provider_executes_onnx_fixture", "--", "--exact", "--ignored"],
        cwd=Path(__file__).resolve().parents[1],
        env={**os.environ, "BOKEHFORGE_ORT_FIXTURE": str(tmp_path), "PYO3_PYTHON": sys.executable},
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    assert "1 passed" in result.stdout, result.stdout
