# RP2040 Benchmark Results

**Last Updated:** 2026-08-01 13:48:21  
**Toolchain:** rustc 1.97.1 (8bab26f4f 2026-07-14)  
**Target:** thumbv6m-none-eabi (Cortex-M0+, no FPU)  
**Optimization:** --release

## Results

```
    Finished `release` profile [optimized] target(s) in 0.19s
     Running `probe-rs run --chip RP2040 --no-timestamps target/thumbv6m-none-eabi/release/benchmark-rp2040`
     Finished in 1.17s

RGB Sequencer Benchmark
=======================

Platform: RP2040 (Cortex-M0+ without FPU)
CPU Frequency: 125 MHz

service() Performance
---------------------
Test Configuration: Time position at last step midpoint

Transition       N=4          N=8          N=16         N=32
Style          cycles/µs    cycles/µs    cycles/µs    cycles/µs
============  ===========  ===========  ===========  ===========
Step            3185/25      5360/42      9716/77     18420/147
Linear          3767/30      5940/47     10739/85     19005/152
EaseIn          3772/30      5949/47     10809/86     19000/152
EaseOut         3775/30      5953/47     11115/88     19013/152
EaseInOut       3780/30      5958/47     10935/87     19019/152

```
