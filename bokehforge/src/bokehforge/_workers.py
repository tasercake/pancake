"""Bounded workers for independent native kernels that release the GIL."""

from concurrent.futures import ThreadPoolExecutor
from functools import lru_cache
import os


@lru_cache(maxsize=1)
def workers():
    count = len(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else os.cpu_count()
    return ThreadPoolExecutor(max_workers=min(8, count or 1),
                              thread_name_prefix="bokehforge")


@lru_cache(maxsize=1)
def alignment_worker():
    # Alignment itself submits native work to workers(). Keep the coordinator
    # separate so queued alignments cannot occupy their own kernel workers.
    return ThreadPoolExecutor(max_workers=1, thread_name_prefix="bokehforge-alignment")
