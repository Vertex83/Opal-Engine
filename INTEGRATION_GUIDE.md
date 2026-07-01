#  Integracja Opal Optimizer do Projektu

## Quick Start (3 Steps)

###  Copy library to project
```bash
# Skopiuj katalog opal_lib/ lub użyj jako git submodule
cp -r /path/to/Opal-Engine /path/to/your_project/opal_lib
```

###  Add to Cargo.toml (if Rust):
```toml
[dependencies]
opal_lib = { path = "../opal_lib" }
```

###  Import and Use:

#### **Rust:**
```rust
use opal_lib::{OptimizationConfig, apply_optimization};

let config = OptimizationConfig::aggressive_safe();
let report = apply_optimization(cpu_load, memory_load, &config);

if report.applied {
    println!("Optimization applied! Priority: {:?}", report.selected_priority);
    println!("Estimated FPS gain: +10–15 FPS");
}
```

#### **C/C++:**
```c
#include "opal_lib.h"

// If Built as cdylib (dynamic library) (dynamic library):
int result = opal_set_priority(1);  // High priority
// Returns: 200 (success), 403 (permissions), 500 (error)

// Lub bardziej kontrolnie:
opal_lib_OptimizationConfig cfg = opal_lib_OptimizationConfig_aggressive_safe();
opal_lib_OptimizationReport report;
opal_apply_optimization(cpu_load, memory_load, &cfg, &report);
```

---

## Full Integration (Recommended)

### Setup in Rust Project:

```rust
use std::time::Instant;
use opal_lib::{OptimizationConfig, apply_optimization, calculate_advanced_optimization};

struct GameOptimizer {
    config: OptimizationConfig,
    last_update: Instant,
}

impl GameOptimizer {
    pub fn new() -> Self {
        Self {
            config: OptimizationConfig::aggressive_safe(),
            last_update: Instant::now(),
        }
    }

    pub fn update(&self, cpu_load: f32, memory_load: f32, num_workers: u32, memory_pressure: f32) {
        // Limit updates to every 100ms (sample_interval)
        if self.last_update.elapsed().as_millis() < self.config.sample_interval_ms as u128 {
            return;
        }

        // Advanced optimization z Thread Pool + Memory Pool
        let report = calculate_advanced_optimization(
            cpu_load,
            memory_load,
            num_workers,
            memory_pressure,
            &self.config,
        );

        if report.base_report.applied {
            println!(" Optimization applied!");
            println!("   Priority: {:?}", report.base_report.selected_priority);
            println!("   Estimated FPS Gain: +{:.1} FPS", report.estimated_fps_gain);
            println!("   Safety Score: {:.2}/1.0", report.safety_score);
            println!("   LOD Culling: {}", if report.lod_culling_enabled { "ON" } else { "OFF" });
        }

        self.last_update = Instant::now();
    }
}

// W game loop:
fn main() {
    let optimizer = GameOptimizer::new();

    loop {
        let cpu_usage = get_cpu_load();  // Your function
        let memory_usage = get_memory_load();  // Your function
        
        optimizer.update(cpu_usage, memory_usage, 8, 0.5);
        
        // Your game loop...
    }
}
```

---

### Setup in C/C++ Project:

#### **CMakeLists.txt:**
```cmake
# Kompiluj Opal jako cdylib (dynamic library)
add_subdirectory(opal_lib)

target_link_libraries(your_app opal_lib)
```

#### **C++ Code:**
```cpp
#include <opal_lib.h>
#include <iostream>
#include <chrono>

class GameOptimizer {
private:
    opal_lib::OptimizationConfig config;
    std::chrono::steady_clock::time_point last_update;

public:
    GameOptimizer() {
        // Use aggressive_safe() zamiast default
        config = opal_lib::OptimizationConfig::aggressive_safe();
        last_update = std::chrono::steady_clock::now();
    }

    void update(float cpu_load, float memory_load, uint32_t num_workers, float memory_pressure) {
        auto now = std::chrono::steady_clock::now();
        if (std::chrono::duration_cast<std::chrono::milliseconds>(now - last_update).count() 
            < config.sample_interval_ms) {
            return;
        }

        opal_lib::AdvancedOptimizationReport report = {};
        int result = opal_lib::opal_advanced_optimize(
            cpu_load, memory_load,
            num_workers, memory_pressure,
            &config,
            &report
        );

        if (result == 0 && report.base_report.applied) {
            std::cout << " Optimization applied!\n";
            std::cout << "   Est. FPS Gain: +" << report.estimated_fps_gain << " FPS\n";
            std::cout << "   Safety Score: " << report.safety_score << "/1.0\n";
        }

        last_update = now;
    }
};

// Game loop:
int main() {
    GameOptimizer optimizer;

    while (running) {
        float cpu = measure_cpu_load();
        float mem = measure_memory_load();
        
        optimizer.update(cpu, mem, 8, 0.5);
        
        // Render & Update...
    }
}
```

---

## Tuning for Your Hardware

### Weak hardware (Intel i5, 8GB RAM):
```rust
let config = OptimizationConfig::aggressive_safe();
// Włącz Thread Pool (4–6 workers)
// Memory Pool: 16–32 MB chunks
```

### Medium hardware (Ryzen 7, 16GB RAM):
```rust
let config = OptimizationConfig::aggressive_safe();
// Thread Pool: 8 workers
// Memory Pool: 64 MB chunks
// Może się sprawdzi nawet ultra_performance()
```

### Powerful hardware (Ryzen 9, 32GB+):
```rust
let config = OptimizationConfig::ultra_performance();
// Thread Pool: 16+ workers
// Memory Pool: 256 MB chunks
//  Requires administrator privileges
```

---

## Monitoring & Diagnostics

### Get last error code:
```c
uint32_t error = opal_last_error();
if (error == 5) {
    printf("Permission denied (run as admin)\n");
} else if (error == 87) {
    printf("Invalid parameter\n");
} else if (error != 0) {
    printf("Windows error: %u\n", error);
}
```

### Logging & Analytics:
```rust
let report = calculate_advanced_optimization(...);

// Zapisz do pliku/bazy dla analiz
if report.estimated_fps_gain > 20.0 && report.safety_score < 0.5 {
    eprintln!("Warning: High FPS gain but low safety score!");
}
```

---

## Best Practices

 **DO:**
- Wywołuj optimization update co 100–200ms (unikaj spam)
- Wybierz **aggressive_safe()** jako default dla nowego projektu
- Monitoruj safety_score i LogWarnings jeśli < 0.3
- Testuj na najsłabszym sprzęcie, na którym ma działać

 **DON'T:**
- Nie uruchamiaj ultra_performance() bez uprawnień administratora
- Nie zmieniaj prioritetu na REALTIME co klatkę (cooldown!)
- Nie ignoruj kodów błędów (403 = uruchom jako Admin)
- Nie ustawiaj cpu_high_threshold poniżej 60% (zbyt agresywne)

---

## Troubleshooting

| Problem | Przyczyna | Rozwiązanie |
|---------|-----------|------------|
| `opal_set_priority()` zwraca 403 | Brak uprawnień administratora | Uruchom `cargo run --release` lub app jako Admin |
| Brak zmian w performance | Prioritet nie zmienia się | Zmień na `aggressive_safe()` |
| System się zawiesa | ultra_performance() + niska RAM | Przejdź na `aggressive_safe()` |
| Kompilacja fails (Windows) | Brak winapi feature'ów | Check Cargo.toml: `features = ["processthreadsapi", "winbase", "errhandlingapi"]` |

---

## Przykład: Pełny Game Loop z Opal

```rust
fn game_loop(mut system: System) {
    let mut optimizer = GameOptimizer::new();
    let mut frame_count = 0u64;

    loop {
        // 1. Update system metrics
        system.refresh_cpu();
        system.refresh_memory();
        
        let cpu_usage = system.global_cpu_info().cpu_usage();
        let memory_usage = (system.used_memory() as f32 / system.total_memory() as f32) * 100.0;

        // 2. Optimize (every 100ms)
        optimizer.update(cpu_usage, memory_usage, 8, 0.5);

        // 3. Render & Physics
        render_frame();
        update_physics(8); // 8 threads
        
        frame_count += 1;

        // 4. Print stats
        if frame_count % 60 == 0 {
            println!("CPU: {:.1}%, MEM: {:.1}%, Opt. Active", cpu_usage, memory_usage);
        }
    }
}
```

---

## Deployment

### Dla DLL/SO:
```bash
# Build as dynamic library
cargo build --release --lib

# Output: target/release/opal_lib.dll (Windows)
#         target/release/libopal_lib.so (Linux)
#         target/release/libopal_lib.dylib (macOS)
```

### Dla Statycznej Biblioteki:
```toml
[lib]
crate-type = ["staticlib"]  # zamiast "cdylib"
```

---

**Done! Twój projekt ma teraz przepotężną, ale bezpieczną optymalizację.** 

*More questions? Sprawdź `OPTIMIZATION_GUIDE.md` lub testach w `tests/optimizer.rs`.*
