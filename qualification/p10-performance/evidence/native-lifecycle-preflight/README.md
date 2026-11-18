# Native lifecycle state visibility

The provisional locked release GUI `0859deb124ed9c9dc4a8dd12a16d309b9af321fc`
ran on the private software Xvfb/AT-SPI desktop. Its binary SHA-256 was
`3326812d97511cedf8ae7aee011f66816fae274f84ae1a1157c5df806d879001`.
The unmodified frozen fixture generated 64 distinct native input paths
with 1024 × 1536 RGB pixels, using its documented shared inode. The user-facing
controls selected Laplacian and Breathing, started the stack, paused and
resumed it, and requested stop through real XTest input.

The observer saw Running in Prepare, worker-confirmed Paused in Prepare,
resumed Running in Fuse, then reusable Idle in Register with one frame
accumulated. The Start control was enabled again. A separately observable
Cancelling acknowledgement was absent from the trace. The worker can settle
between GUI polls. A timing sample needs a stable native record of the actual
worker acknowledgement under the same generation, with no artificial dwell.

The first setup used a case-sensitive raw native selector with lowercase
`breathing`. The actual popup exposed `Breathing`, so that preparation ended
before a job began. The unchanged semantic adapter maps this name.

The application timestamps were excluded from this check. Both generated
fixture directories were removed after use. The physical GPU remained idle
from these software-presented CPU tasks.
