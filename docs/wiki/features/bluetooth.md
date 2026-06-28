# Bluetooth

Discover, pair, connect, and manage Bluetooth devices.

## Actions

### bluetooth.list

List known (paired or connected) Bluetooth devices.

```bash
deskbrid bluetooth.list
```

No parameters.

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {"name": "Sony WH-1000XM4", "address": "XX:XX:XX:XX:XX:XX", "connected": true, "paired": true, "device_class": "headset"},
    {"name": "Magic Mouse", "address": "YY:YY:YY:YY:YY:YY", "connected": false, "paired": true, "device_class": "mouse"}
  ]
}
```

### bluetooth.scan

Scan for discoverable Bluetooth devices within range.

| Parameter  | Type   | Description                    |
|------------|--------|--------------------------------|
| `duration` | uint?  | Scan duration in seconds (default: 10) |

```bash
deskbrid bluetooth.scan '{"duration": 15}'
```

```json
{
  "type": "bluetooth.scan",
  "duration": 15
}
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {"name": "iPhone", "address": "ZZ:ZZ:ZZ:ZZ:ZZ:ZZ", "device_class": "phone", "rssi": -65}
  ]
}
```

### bluetooth.stop_scan

Stop an active Bluetooth scan.

```bash
deskbrid bluetooth.stop_scan
```

No parameters.

### bluetooth.connect

Connect to a paired Bluetooth device.

| Parameter | Type   | Description      |
|-----------|--------|------------------|
| `address` | string | Device MAC address |

```bash
deskbrid bluetooth.connect '{"address": "XX:XX:XX:XX:XX:XX"}'
```

```json
{
  "type": "bluetooth.connect",
  "address": "XX:XX:XX:XX:XX:XX"
}
```

### bluetooth.disconnect

Disconnect a connected Bluetooth device.

| Parameter | Type   | Description      |
|-----------|--------|------------------|
| `address` | string | Device MAC address |

```bash
deskbrid bluetooth.disconnect '{"address": "XX:XX:XX:XX:XX:XX"}'
```

### bluetooth.pair

Initiate pairing with a discovered Bluetooth device.

| Parameter | Type   | Description      |
|-----------|--------|------------------|
| `address` | string | Device MAC address |

```bash
deskbrid bluetooth.pair '{"address": "ZZ:ZZ:ZZ:ZZ:ZZ:ZZ"}'
```

Some devices require a pairing confirmation (PIN or numeric comparison). The
agent will report the required code through the confirmation mode workflow.

### bluetooth.forget

Remove (unpair) a Bluetooth device.

| Parameter | Type   | Description      |
|-----------|--------|------------------|
| `address` | string | Device MAC address |

```bash
deskbrid bluetooth.forget '{"address": "YY:YY:YY:YY:YY:YY"}'
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# List known devices
devices = client.bluetooth_list()
for d in devices:
    print(f"{d['name']} - {'connected' if d['connected'] else 'disconnected'}")

# Scan for new devices
found = client.bluetooth_scan(duration=10)
for d in found:
    print(f"Found: {d['name']} ({d['address']})")

# Pair and connect
if found:
    client.bluetooth_pair(address=found[0]["address"])
    client.bluetooth_connect(address=found[0]["address"])
```

## Requirements

- BlueZ D-Bus API (`org.bluez`). Requires `bluetoothd` running.
- Pairing may require a Bluetooth agent to handle PIN/confirmation codes.
- Scanning requires Bluetooth adapter in discoverable mode.

## Current Status

**Stable** — list, scan, connect, disconnect.
**Experimental** — pair, forget.
