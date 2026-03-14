# Opal Engine
<img src="assets/opallogo.png" alt="Opal Logo" width="">

## Description
Opal is a high-performance optimization engine written in Rust, designed to manage process priorities in Windows environments. It ensures stability for performance-critical applications (like games) by optimizing thread scheduling.

## Key Features
- **High Priority Enforcement:** Forces the "High" priority class on your process to ensure consistent CPU access.
- **Low Overhead:** Engineered to be lightweight, maintaining a minimal resource footprint (typically adding only ~1% CPU usage compared to an idle system).
- **Frequency Stability:** Helps stabilize CPU clock speed by preventing chaotic task switching.

## How it works
Opal communicates directly with the Windows API (`SetPriorityClass`), allowing for precise control over resource allocation without modifying the target application's source code.



## How to run
1. Ensure the `opal_lib.dll` file is in the same directory as your Python script.
2. Execute your script using: `python game.py`.
3. Monitor your process in Task Manager (Details tab) to verify the "High" priority status.

## Documentation
- **Architecture:** x86_64 / Windows API.
- **Language:** Rust (core) / Python (interface).

## Status
- Currently in active development.

## License
This project is licensed under the terms of the Proprietary License Agreement. 
See the [LICENSE](LICENSE) file for details.

## Contact & Support
If you have questions, found a bug, or want to cooperate:
- **GitHub Issues:** https://github.com/Vertex83/Opal-Engine/issues
- **Discord:** @vertexch