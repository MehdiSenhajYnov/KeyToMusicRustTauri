| experiment | model | quantization | runtime | prep | decision | strict | delta_strict | relaxed | delta_relaxed | intensity | errors | avg_window_s | vram_mib | weights_mib |
|---|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| t4b_wide5_selective_hold_runtime_gpu_32 | Qwen3-VL-4B-Thinking | llm=Q4_K_M + mmproj=F16 | gpu_12288x1_ngl32 [c=12288 np=1 ngl=32] | legacy_672_jpeg | wide5_selective_repair_hold | 0/1 | +0 | 1/1 | +0 | 0/1 | 0 | 54.3 | 6883 | 3180 |
