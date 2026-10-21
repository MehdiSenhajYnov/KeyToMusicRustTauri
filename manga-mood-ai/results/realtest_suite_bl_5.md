| experiment | model | quantization | runtime | prep | decision | strict | delta_strict | relaxed | delta_relaxed | intensity | errors | avg_window_s | vram_mib | weights_mib |
|---|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| t4b_wide5_selective_hold | Qwen3-VL-4B-Thinking | llm=Q4_K_M + mmproj=F16 | prod_8192x1 [c=8192 np=1 ngl=48] | legacy_672_jpeg | wide5_selective_repair_hold | 15/21 | +0 | 19/21 | +0 | 3/21 | 0 | 9.9 | 6632 | 3180 |
