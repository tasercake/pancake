# Independent reading: input precision and radiometry

Accessed 2026-11-15. Root checked code snapshots acquired by the algorithm-review agent. Static inspection only; no baseline executed.

- [focus-stack input loader at pinned commit](https://github.com/PetteriAimonen/focus-stack/blob/0cd289e224fe24aead5b146faeaa46950775f346/src/task_loadimg.cc#L53) requests `IMREAD_ANYCOLOR`, without `IMREAD_ANYDEPTH`. [Color reassignment](https://github.com/PetteriAimonen/focus-stack/blob/0cd289e224fe24aead5b146faeaa46950775f346/src/task_reassign.cc#L56) explicitly asserts 8-bit input types and allocates 8-bit color output. File-format support does not establish high-bit-depth processing.
- The same loader requests 16-bit RAW postprocessing with camera white balance and disabled automatic brightness, but does not explicitly request linear gamma. Its exception fallback can use an embedded preview or lower-bit-depth postprocessing. Native loading should therefore be evaluated separately from a fixed external RAW/color conversion path.

Implication: maintain separate native-pipeline and controlled-input comparisons. Record decoded signal type, range, transfer function, fallback events and output quantization before interpreting quality differences.
