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

## Durable acknowledgement recheck

The locked release GUI
`bc192616ecebad47fef0ced154ca4361c2f0b994`, actual binary SHA-256
`57053ae2b7e15e4e5f21559149b0e8dc17d43dec7f218169c799ce194e5b3598`,
passed the same untimed private 64-frame route after native metadata gained
the actual worker acknowledgement. Before Stop, native Generation(1)
reported Running in Register with one input complete and
`cancel_acknowledged=false`. The first acknowledgement observation came
through native Idle in Decode with three inputs complete,
`cancel_acknowledged=true`, and the same Generation(1). Start was enabled.
The first native acknowledgement and settled Idle were visible together,
while the worker's actual acknowledgement marker retained their distinct
semantics.

The metadata was derived from the live worker snapshot. The test introduced
no worker dwell or timing hook. Temporary inputs were removed. No latency
interval was retained or counted toward the frozen target.
