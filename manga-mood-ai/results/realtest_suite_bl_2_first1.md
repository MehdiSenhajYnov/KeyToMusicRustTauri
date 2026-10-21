| experiment | model | quantization | runtime | prep | decision | strict | delta_strict | relaxed | delta_relaxed | intensity | errors | avg_window_s | vram_mib | weights_mib |
|---|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| t4b_wide5_selective_hold | Qwen3-VL-4B-Thinking | llm=Q4_K_M + mmproj=F16 | prod_8192x1 [c=8192 np=1 ngl=48] | legacy_672_jpeg | wide5_selective_repair_hold | 0/1 | +0 | 1/1 | +0 | 0/1 | 0 | 5.5 | 6630 | 3180 |
