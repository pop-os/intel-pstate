# intel-pstate

Rust crate for fetching and modifying intel_pstate kernel parameters.

```rust
use std::io;
use intel_pstate::PState;

fn main() -> io::Result<()> {
    if let Ok(pstate) = PState::new() {
        pstate.set_min_perf_pct(50)?;
        pstate.set_max_perf_pct(100)?;
        pstate.set_no_turbo(false)?;
    }

    Ok(())
}
```