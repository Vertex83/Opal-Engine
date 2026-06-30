# C++ integration example

Build the Rust library first:

```powershell
cargo build --release
```

Then link the generated `target/release/opal_lib.dll` from your C++ project and include `cpp_integration.cpp`.
