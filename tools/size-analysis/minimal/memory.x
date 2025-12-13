/* Linker script for minimal size analysis */
/* Generic Cortex-M0 memory layout */

MEMORY
{
  /* 64KB Flash, 8KB RAM - conservative minimal target */
  FLASH : ORIGIN = 0x00000000, LENGTH = 64K
  RAM   : ORIGIN = 0x20000000, LENGTH = 8K
}

/* This is required by cortex-m-rt */
_stack_start = ORIGIN(RAM) + LENGTH(RAM);
