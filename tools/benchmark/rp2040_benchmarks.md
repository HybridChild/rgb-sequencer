# RP2040 Benchmark Results

**Last Updated:** 2025-12-15 17:40:46  
**Toolchain:** rustc 1.91.1 (ed61e7d7e 2025-11-07)  
**Target:** thumbv6m-none-eabi (Cortex-M0+, no FPU)  
**Optimization:** --release

## Results

```
    Finished `release` profile [optimized] target(s) in 0.18s
     Running `probe-rs run --chip RP2040 --no-timestamps target/thumbv6m-none-eabi/release/benchmark-rp2040`
     Finished in 2.07s

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
Step            4020/32      6370/50     10476/83     19087/152
Linear          6053/48      9253/74     12557/100    21161/169
EaseIn          6163/49      9358/74     12668/101    21276/170
EaseOut         6290/50      9490/75     12801/102    21406/171
EaseInOut       6559/52      9760/78     13066/104    21670/173

```
