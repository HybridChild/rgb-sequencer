# RP2040 Benchmark Results

**Last Updated:** 2025-12-16 10:15:13  
**Toolchain:** rustc 1.91.1 (ed61e7d7e 2025-11-07)  
**Target:** thumbv6m-none-eabi (Cortex-M0+, no FPU)  
**Optimization:** --release

## Results

```
    Finished `release` profile [optimized] target(s) in 0.17s
     Running `probe-rs run --chip RP2040 --no-timestamps target/thumbv6m-none-eabi/release/benchmark-rp2040`
     Finished in 2.05s

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
Step            4018/32      6170/49     10475/83     20331/162
Linear          6050/48      8301/66     12572/100    22196/177
EaseIn          6175/49      8405/67     12675/101    22301/178
EaseOut         6308/50      8535/68     12812/102    22432/179
EaseInOut       6576/52      8805/70     13075/104    22702/181
EaseOutIn       6656/53      8885/71     13155/105    22763/182

```
