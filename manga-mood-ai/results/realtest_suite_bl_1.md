| experiment | model | quantization | runtime | prep | decision | strict | delta_strict | relaxed | delta_relaxed | intensity | errors | avg_window_s | vram_mib | weights_mib |
|---|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| t4b_wide5_selective_hold_runtime_gpu_64 | Qwen3-VL-4B-Thinking | llm=Q4_K_M + mmproj=F16 | gpu_12288x1_ngl64 [c=12288 np=1 ngl=64] | legacy_672_jpeg | wide5_selective_repair_hold | 59/70 | +0 | 68/70 | +0 | 24/70 | 0 | 13.3 | 7250 | 3180 |
