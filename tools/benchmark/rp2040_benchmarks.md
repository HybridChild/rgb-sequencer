# RP2040 Benchmark Results

**Last Updated:** 2025-12-16 19:59:06  
**Toolchain:** rustc 1.91.1 (ed61e7d7e 2025-11-07)  
**Target:** thumbv6m-none-eabi (Cortex-M0+, no FPU)  
**Optimization:** --release

## Results

```
    Finished `release` profile [optimized] target(s) in 0.03s
     Running `probe-rs run --chip RP2040 --no-timestamps target/thumbv6m-none-eabi/release/benchmark-rp2040`
     Finished in 1.63s

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
Step            4248/33      6933/55     10690/85     19286/154
Linear          5993/47      8758/70     12488/99     21076/168
EaseIn          6100/48      8862/70     12585/100    21180/169
EaseOut         6230/49      8988/71     12721/101    21316/170
EaseInOut       6500/52      9259/74     12985/103    21580/172
EaseOutIn       6575/52      9341/74     13065/104    21660/173

```
