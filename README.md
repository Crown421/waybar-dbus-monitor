# waybar-dbus-monitor

A command-line tool for monitoring D-Bus signals and formatting output for waybar and other status bars.

## Usage

```bash
waybar-dbus-monitor --interface <INTERFACE> --member <MEMBER> <TYPE>
```

### Example

Monitor a boolean D-Bus signal with custom icons:

```bash
waybar-dbus-monitor --interface org.guayusa.Idle --member StatusChanged boolean --return-true "󰈈" --return-false "󰈉"
```

### Options

- `--interface`: D-Bus interface to monitor
- `--member`: D-Bus member (signal/method) to monitor

### Type Handlers

#### Boolean
Monitor boolean values and return custom strings:
- `--return-true`: String to output when value is true (default: "true")
- `--return-false`: String to output when value is false (default: "false")

## Building

```bash
cargo build --release
```

## Debugging

To enable debug logging, set the `RUST_LOG` environment variable before running the command:

```bash
RUST_LOG=debug waybar-dbus-monitor --interface org.example.Interface --member Signal boolean
```

This will show detailed information about D-Bus connections, match rules, and signal processing.

## Status

🚧 **Work in Progress** - Currently implements CLI parsing. D-Bus monitoring functionality coming soon.
