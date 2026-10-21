| experiment | model | quantization | prep | decision | strict | delta_strict | relaxed | delta_relaxed | intensity | avg_window_s | vram_mib | weights_mib |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| t4b_wide5_selective_hold | Qwen3-VL-4B-Thinking | llm=Q4_K_M + mmproj=F16 | legacy_672_jpeg | wide5_selective_repair_hold | 59/70 | +0 | 68/70 | +0 | 24/70 | 12.1 | 6437 | 3180 |
| t4b_wide5_selective_hold_mmproj_q8_0 | Qwen3-VL-4B-Thinking | llm=Q4_K_M + mmproj=Q8_0 | legacy_672_jpeg | wide5_selective_repair_hold | 47/70 | -12 | 61/70 | -7 | 22/70 | 11.0 | 6567 | 2815 |
| t4b_wide5_selective_hold_q8_0 | Qwen3-VL-4B-Thinking | llm=Q8_0 + mmproj=F16 | legacy_672_jpeg | wide5_selective_repair_hold | 49/70 | -10 | 63/70 | -5 | 24/70 | 15.5 | 8264 | 4880 |
| t4b_wide5_selective_hold_f16 | Qwen3-VL-4B-Thinking | llm=F16 + mmproj=F16 | legacy_672_jpeg | wide5_selective_repair_hold | 47/70 | -12 | 65/70 | -3 | 23/70 | 23.7 | 11747 | 8476 |
