# Integracja do dowolnego projektu

1. Skopiuj folder `src`, `Cargo.toml`, `config` oraz `examples` do projektu.
2. Dodaj zależność Rust do swojego build systemu lub zbuduj bibliotekę DLL:
   ```powershell
   cargo build --release
   ```
3. W C/C++ podłącz biblioteki:
   - `target/release/opal_lib.dll`
   - `examples/cpp_integration.cpp`
4. W Rust możesz użyć modułów z `src/lib.rs` jako część własnej biblioteki.

Najbardziej bezpieczny tryb pracy:
- nie ustawiaj `Realtime` zbyt agresywnie,
- zostaw `cooldown_ms` i `smoothing_factor`,
- uruchamiaj optymalizację co kilka setek ms.
