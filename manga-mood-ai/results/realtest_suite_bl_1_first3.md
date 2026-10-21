| experiment | model | quantization | runtime | prep | decision | strict | delta_strict | relaxed | delta_relaxed | intensity | errors | avg_window_s | vram_mib | weights_mib |
|---|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| gemma4_e4b_wide5_selective_hold | Gemma 4 E4B | llm=Q4_K_M + mmproj=F16 | prod_8192x1 [c=8192 np=1 ngl=auto] | legacy_672_jpeg | wide5_selective_repair_hold | 0/0 | +0 | 0/0 | +0 | 0/0 | 1 | 0.0 | n/a | 5692 |
