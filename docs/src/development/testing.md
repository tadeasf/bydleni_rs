# Testing

## Unit tests

```bash
cargo test
```

31 tests covering:

- **Mortgage calculation**: standard and zero-rate scenarios
- **Area/layout extraction**: parsing Sreality listing names
- **Czech tax**: gross-to-net wage conversion
- **Savings calculator**: various input combinations
- **Living expenses**: regional lookup
- **Linear regression**: statistical functions
- **Forecast**: slope calculation and extrapolation
- **Scenario**: input validation, computation, sorting, severity colors

## Linting

```bash
cargo clippy -- -D warnings
```

Must pass clean (zero warnings as errors).

## Formatting

```bash
cargo fmt --check
```

Uses project `rustfmt.toml` settings:
- `max_width = 100`
- `use_small_heuristics = "Max"`
