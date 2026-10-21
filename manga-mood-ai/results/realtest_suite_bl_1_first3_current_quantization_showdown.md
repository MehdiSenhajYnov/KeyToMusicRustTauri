| experiment | model | quantization | prep | decision | strict | delta_strict | relaxed | delta_relaxed | intensity | avg_window_s | vram_mib | weights_mib |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| t4b_wide5_selective_hold | Qwen3-VL-4B-Thinking | llm=Q4_K_M + mmproj=F16 | legacy_672_jpeg | wide5_selective_repair_hold | 1/1 | +0 | 1/1 | +0 | 0/1 | 10.8 | 6465 | 3180 |
| t4b_wide5_selective_hold_mmproj_q8_0 | Qwen3-VL-4B-Thinking | llm=Q4_K_M + mmproj=Q8_0 | legacy_672_jpeg | wide5_selective_repair_hold | 0/1 | -1 | 1/1 | +0 | 0/1 | 13.5 | 6096 | 2815 |
| t4b_wide5_selective_hold_q8_0 | Qwen3-VL-4B-Thinking | llm=Q8_0 + mmproj=F16 | legacy_672_jpeg | wide5_selective_repair_hold | 0/1 | -1 | 1/1 | +0 | 0/1 | 14.1 | 8192 | 4880 |
| t4b_wide5_selective_hold_f16 | thinking4b_f16 | unavailable | legacy_672_jpeg | wide5_selective_repair_hold | 0/0 | -1 | 0/0 | -1 | 0/0 | 0.0 | n/a | n/a |
