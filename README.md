# waybar-dbus-monitor

A command-line tool for monitoring D-Bus signals and formatting output for waybar and other status bars.

## Usage

```bash
waybar-dbus-monitor --interface <INTERFACE> --monitor <MONITOR> [--status "service/path interface property"] <TYPE>
```

### Example

Monitor a boolean D-Bus signal with custom icons:

```bash
waybar-dbus-monitor --interface org.guayusa.Idle --monitor StatusChanged boolean --return-true "󰈈" --return-false "󰈉"
```

Check a property at startup and then monitor signals:

```bash
# With root object path
waybar-dbus-monitor --interface org.guayusa.Idle --monitor StatusChanged --status "org.guayusa.IdleInhibitor/ org.guayusa.Idle Status" boolean --return-true "󰈈" --return-false "󰈉"

# With full object path
waybar-dbus-monitor --interface org.example.Test --monitor TestSignal --status "org.example.Service/org/example/Object org.example.Interface TestProperty" boolean --return-true "󰈈" --return-false "󰈉"
```

### Options

- `--interface`: D-Bus interface and service name to monitor
- `--monitor`: D-Bus member (signal/method) to monitor
- `--status`: (Optional) Initial status check in format "service/path interface property". The format must be exactly three whitespace-separated tokens with no spaces in the service/path part.

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
RUST_LOG=debug waybar-dbus-monitor --interface org.example.Interface --monitor Signal boolean
```

This will show detailed information about D-Bus connections, match rules, and signal processing.

## Status

🚧 **Work in Progress** - Currently implements CLI parsing. D-Bus monitoring functionality coming soon.
