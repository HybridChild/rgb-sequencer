# RP2040 Benchmark Results

**Last Updated:** 2026-08-01 09:26:38  
**Toolchain:** rustc 1.97.1 (8bab26f4f 2026-07-14)  
**Target:** thumbv6m-none-eabi (Cortex-M0+, no FPU)  
**Optimization:** --release

## Results

```
    Finished `release` profile [optimized] target(s) in 0.20s
     Running `probe-rs run --chip RP2040 --no-timestamps target/thumbv6m-none-eabi/release/benchmark-rp2040`
     Finished in 1.15s

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
Step            3171/25      5340/42      9679/77     18351/146
Linear          3755/30      5927/47     10601/84     18940/151
EaseIn          3760/30      5931/47     10604/84     18943/151
EaseOut         3764/30      5935/47     10613/84     18947/151
EaseInOut       3769/30      5939/47     10614/84     18952/151
EaseOutIn       3775/30      5942/47     10618/84     18955/151

```
