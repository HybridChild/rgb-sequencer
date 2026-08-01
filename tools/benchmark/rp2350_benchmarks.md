# RP2350 Benchmark Results

**Last Updated:** 2026-08-01 13:50:02  
**Toolchain:** rustc 1.97.1 (8bab26f4f 2026-07-14)  
**Target:** thumbv8m.main-none-eabihf (Cortex-M33, with FPU)  
**Optimization:** --release

## Results

```
    Finished `release` profile [optimized] target(s) in 0.29s
     Running `probe-rs run --chip RP235x --no-timestamps target/thumbv8m.main-none-eabihf/release/benchmark-rp2350`
     Finished in 1.16s

RGB Sequencer Benchmark
=======================

Platform: RP2350 (Cortex-M33F with FPU)
CPU Frequency: 150 MHz

service() Performance
---------------------
Test Configuration: Time position at last step midpoint

Transition       N=4          N=8          N=16         N=32
Style          cycles/µs    cycles/µs    cycles/µs    cycles/µs
============  ===========  ===========  ===========  ===========
Step            1705/11      2898/19      5280/35     10053/67 
Linear          1989/13      3180/21      5565/37     10337/68 
EaseIn          1995/13      3180/21      5572/37     10341/68 
EaseOut         1996/13      3188/21      5580/37     10337/68 
EaseInOut       2000/13      3190/21      5580/37     10330/68 

```
