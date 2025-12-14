# RP2350 Benchmark Results

**Last Updated:** 2025-12-15 17:21:43  
**Toolchain:** rustc 1.91.1 (ed61e7d7e 2025-11-07)  
**Target:** thumbv8m.main-none-eabihf (Cortex-M33, with FPU)  
**Optimization:** --release

## Results

```
    Finished `release` profile [optimized] target(s) in 0.15s
     Running `probe-rs run --chip RP235x --no-timestamps target/thumbv8m.main-none-eabihf/release/benchmark-rp2350`
     Finished in 2.09s

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
Step            1789/11      2981/19      5356/35     10134/67 
Linear          2131/14      3322/22      5706/38     10474/69 
EaseIn          2135/14      3324/22      5710/38     10470/69 
EaseOut         2136/14      3324/22      5712/38     10480/69 
EaseInOut       2138/14      3351/22      5722/38     10494/69 

```
