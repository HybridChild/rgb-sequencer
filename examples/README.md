# Examples

Platform-specific examples demonstrating `rgb-sequencer` usage on embedded hardware.

## Available Platforms

### [STM32F0 (Bare-Metal)](stm32f0/)
Bare-metal examples for STM32F NUCLEO-F072RB using `stm32f0xx-hal` and SysTick timing.

**Examples:**
- `blinky` - Simple blocking RGB LED sequencing
- `mode_switcher` - Mode switching with function-based breathing sequence
- `rainbow_capture` - Multi-LED color capture with pause/resume

### [STM32F0-Embassy (Async)](stm32f0-embassy/)
Async examples for STM32F NUCLEO-F072RB using Embassy runtime and async tasks.

**Examples:**
- `mode_switcher` - Single-LED control with async tasks and channels
- `rainbow_capture` - Multi-LED heterogeneous collection with enum wrapper pattern
