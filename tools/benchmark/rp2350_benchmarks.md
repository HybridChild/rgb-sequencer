# RP2350 Benchmark Results

**Last Updated:** 2025-12-16 19:57:07  
**Toolchain:** rustc 1.91.1 (ed61e7d7e 2025-11-07)  
**Target:** thumbv8m.main-none-eabihf (Cortex-M33, with FPU)  
**Optimization:** --release

## Results

```
    Finished `release` profile [optimized] target(s) in 0.03s
     Running `probe-rs run --chip RP235x --no-timestamps target/thumbv8m.main-none-eabihf/release/benchmark-rp2350`
     Finished in 1.61s

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
Step            1801/12      2984/19      5369/35     10134/67 
Linear          2133/14      3324/22      5709/38     10477/69 
EaseIn          2136/14      3327/22      5700/38     10480/69 
EaseOut         2139/14      3330/22      5712/38     10481/69 
EaseInOut       2151/14      3340/22      5723/38     10493/69 
EaseOutIn       2154/14      3342/22      5730/38     10497/69 

```
