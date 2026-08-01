# RP2350 Benchmark Results

**Last Updated:** 2026-08-01 09:25:10  
**Toolchain:** rustc 1.97.1 (8bab26f4f 2026-07-14)  
**Target:** thumbv8m.main-none-eabihf (Cortex-M33, with FPU)  
**Optimization:** --release

## Results

```
    Finished `release` profile [optimized] target(s) in 0.24s
     Running `probe-rs run --chip RP235x --no-timestamps target/thumbv8m.main-none-eabihf/release/benchmark-rp2350`
     Finished in 1.17s

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
Step            1704/11      2892/19      5280/35     10052/67 
Linear          1989/13      3186/21      5567/37     10339/68 
EaseIn          1993/13      3187/21      5571/37     10349/68 
EaseOut         1995/13      3186/21      5579/37     10336/68 
EaseInOut       1999/13      3193/21      5550/37     10349/68 
EaseOutIn       1980/13      3195/21      5581/37     10354/69 

```

**Note:** The `EaseOutIn` row predates 0.3.0, which removed that transition style. The capture is left as recorded.
