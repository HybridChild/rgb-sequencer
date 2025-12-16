# RP2350 Benchmark Results

**Last Updated:** 2025-12-16 10:17:39  
**Toolchain:** rustc 1.91.1 (ed61e7d7e 2025-11-07)  
**Target:** thumbv8m.main-none-eabihf (Cortex-M33, with FPU)  
**Optimization:** --release

## Results

```
    Finished `release` profile [optimized] target(s) in 0.15s
     Running `probe-rs run --chip RP235x --no-timestamps target/thumbv8m.main-none-eabihf/release/benchmark-rp2350`
     Finished in 2.04s

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
Step            1789/11      2982/19      5356/35     10135/67 
Linear          2131/14      3323/22      5706/38     10478/69 
EaseIn          2133/14      3325/22      5730/38     10480/69 
EaseOut         2136/14      3330/22      5712/38     10482/69 
EaseInOut       2148/14      3338/22      5722/38     10494/69 
EaseOutIn       2154/14      3340/22      5725/38     10498/69 

```
