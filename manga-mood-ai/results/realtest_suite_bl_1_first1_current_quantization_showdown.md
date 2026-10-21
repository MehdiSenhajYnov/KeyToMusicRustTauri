| experiment | model | quantization | prep | decision | strict | delta_strict | relaxed | delta_relaxed | intensity | avg_window_s | vram_mib | weights_mib |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| t4b_wide5_selective_hold | Qwen3-VL-4B-Thinking | llm=Q4_K_M + mmproj=F16 | legacy_672_jpeg | wide5_selective_repair_hold | 0/0 | +0 | 0/0 | +0 | 0/0 | 11.3 | 6412 | 3180 |
| t4b_wide5_selective_hold_mmproj_q8_0 | thinking4b_mmproj_q8_0 | unavailable | legacy_672_jpeg | wide5_selective_repair_hold | 0/0 | +0 | 0/0 | +0 | 0/0 | 0.0 | n/a | n/a |
| t4b_wide5_selective_hold_q8_0 | thinking4b_q8_0 | unavailable | legacy_672_jpeg | wide5_selective_repair_hold | 0/0 | +0 | 0/0 | +0 | 0/0 | 0.0 | n/a | n/a |
| t4b_wide5_selective_hold_f16 | thinking4b_f16 | unavailable | legacy_672_jpeg | wide5_selective_repair_hold | 0/0 | +0 | 0/0 | +0 | 0/0 | 0.0 | n/a | n/a |
