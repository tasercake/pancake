"""Bounded workers for independent native kernels that release the GIL."""

from concurrent.futures import ThreadPoolExecutor
from functools import lru_cache
import os


@lru_cache(maxsize=1)
def workers():
    count = len(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else os.cpu_count()
    return ThreadPoolExecutor(max_workers=min(8, count or 1),
                              thread_name_prefix="bokehforge")
