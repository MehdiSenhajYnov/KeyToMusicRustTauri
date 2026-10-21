| experiment | model | quantization | runtime | prep | decision | strict | delta_strict | relaxed | delta_relaxed | intensity | errors | avg_window_s | vram_mib | weights_mib |
|---|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| gemma4_e4b_wide5_selective_hold | Gemma 4 E4B | llm=Q4_K_M + mmproj=F16 | prod_8192x1 [c=8192 np=1 ngl=24] | legacy_672_jpeg | wide5_selective_repair_hold | 0/0 | +0 | 0/0 | +0 | 0/0 | 0 | 28.2 | 7231 | 5692 |
