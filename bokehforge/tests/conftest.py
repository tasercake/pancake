from collections import Counter, defaultdict

import pytest


_ACCELERATORS = {"cuda": "CUDA", "mps": "Metal"}


@pytest.fixture
def score_device(request):
    device = request.param
    if device == "cpu":
        return device

    from bokehforge._device import damage_backend

    name = _ACCELERATORS[device]
    request.node.user_properties.append(("hardware_accelerator", name))
    try:
        return damage_backend(device)
    except RuntimeError as error:
        pytest.skip(f"{name} unavailable ({error})")


def pytest_terminal_summary(terminalreporter):
    devices = defaultdict(Counter)
    for outcome in ("passed", "failed", "skipped", "error"):
        for report in terminalreporter.stats.get(outcome, ()):
            for key, name in getattr(report, "user_properties", ()):
                if key == "hardware_accelerator":
                    devices[name][outcome] += 1
    if devices:
        terminalreporter.write_sep("-", "Hardware accelerator tests")
        for name, outcomes in sorted(devices.items()):
            failed = outcomes["failed"] + outcomes["error"]
            terminalreporter.write_line(
                f"{name}  {outcomes['passed']} passed, {outcomes['skipped']} skipped, {failed} failed"
            )
