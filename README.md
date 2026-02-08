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
cargo run receive 8080

# Start the Satellite OCS (Sender)
cargo run send localhost 8080
```

## Performance Monitoring
The simulation logs the following metrics for analysis:
*   **Scheduling Drift:** Difference between scheduled and actual start times.
*   **Pipeline Latency:** Time from sensor read to buffer/uplink.
*   **Jitter:** Variation in periodic task timing.
*   **CPU Utilization:** Active vs. Idle time.

## License
Internal University Project - Asia Pacific University.
