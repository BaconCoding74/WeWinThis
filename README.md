# Satellite-Ground Control Real-Time System Simulation (Rust)

This project is an implementation of a real-time system simulation for the **Realtime Systems (CT087-3-3)** module at Asia Pacific University. It simulates the coordination between a CubeSat Onboard Control System (OCS) and a Ground Control Station (GCS).

## Project Overview

The system consists of two primary components:
1.  **Student A: Satellite Onboard Control System (OCS)**
    *   Sensor Data Acquisition (Thermal, Power, Navigation).
    *   Real-time Task Scheduling (RM/EDF).
    *   Downlink Data Management.
    *   Fault Injection and Recovery.
2.  **Student B: Ground Control Station (GCS)**
    *   Telemetry Reception and Decoding.
    *   Command Uplink Scheduling with Safety Interlocks.
    *   Fault Management and Monitoring.

## Key Real-Time Constraints

| Metric | Target |
| :--- | :--- |
| Critical Sensor Jitter | < 1ms |
| Fault Recovery Time | < 200ms (OCS) / < 100ms (GCS) |
| Telemetry Decoding Latency | < 3ms |
| Command Dispatch Latency | < 2ms |
| Downlink Initialization | < 5ms |

## Getting Started

### Prerequisites
*   Rust (latest stable)
*   Cargo

### Running the Prototype
Currently, the system provides a simple UDP-based communication prototype:

```bash
# Start the Ground Control Station (Receiver)
cargo run --bin WeWinThis -- receive 8080

# Start the Satellite OCS (Sender)
cargo run --bin WeWinThis -- send localhost 8080
```

### Mock OCS Simulator for GCS Testing
For Student B (GCS implementation), a **Mock OCS Simulator** is available to simulate OCS telemetry responses without needing the actual OCS code:

```bash
# Start the GCS receiver first
cargo run --bin WeWinThis -- receive 8080

# In another terminal, run the Mock OCS simulator:
# Normal mode - realistic telemetry
cargo run --bin mock_ocs -- localhost 8080 normal 100 1000

# Edge case mode - test extreme conditions
cargo run --bin mock_ocs -- localhost 8080 edge 20 500

# Mixed mode - normal + edge cases (10% edge cases)
cargo run --bin mock_ocs -- localhost 8080 mixed 100 500 0.1

# Continuous mode - infinite streaming (Ctrl+C to stop)
cargo run --bin mock_ocs -- localhost 8080 continuous 1000
```

#### Mock OCS Command Syntax
```bash
cargo run --bin mock_ocs -- <host> <port> [mode] [args]
```

| Mode | Arguments | Description |
|------|-----------|-------------|
| `normal` | `<count> <interval_ms>` | Generate realistic telemetry (default: 100 packets, 1000ms) |
| `edge` | `<count> <interval_ms>` | Inject edge cases only (extreme temps, low battery, etc.) |
| `mixed` | `<count> <interval_ms> <ratio>` | Combined normal + edge cases (ratio: 0.0-1.0) |
| `continuous` | `<interval_ms>` | Infinite streaming mode |

#### Edge Cases Simulated
- Extreme temperatures: -50°C (cold), 125°C (hot)
- Low battery: 2000mV (warning), 0mV (critical)
- Antenna misalignment: -90°, 90° (extreme angles)

#### Telemetry Packet Format (14 bytes)
```
timestamp_ms: u64  (8 bytes)
temperature:  i16  (2 bytes)
battery_mv:   u16  (2 bytes)
antenna_angle: i16 (2 bytes)
```

#### Performance Metrics Reported
- Packets sent/count
- Total bytes transferred
- Average send latency (μs)
- Min/Max send latency
- Packets/second rate
- Edge cases injected count

## Performance Monitoring
The simulation logs the following metrics for analysis:
*   **Scheduling Drift:** Difference between scheduled and actual start times.
*   **Pipeline Latency:** Time from sensor read to buffer/uplink.
*   **Jitter:** Variation in periodic task timing.
*   **CPU Utilization:** Active vs. Idle time.

## License
Internal University Project - Asia Pacific University.
