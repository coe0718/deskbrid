# Bluetooth

Discover and control Bluetooth devices.

## List Devices

```bash
deskbrid bluetooth list
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {
      "name": "My Headphones",
      "mac": "00:11:22:33:44:55",
      "connected": true,
      "paired": true
    }
  ]
}
```

Protocol:
```json
{"action": "bluetooth.list"}
```

## Connect/Disconnect

```bash
deskbrid bluetooth connect 00:11:22:33:44:55
deskbrid bluetooth disconnect 00:11:22:33:44:55
```

Protocol:
```json
{"action": "bluetooth.connect", "mac": "00:11:22:33:44:55"}
```

## Pair Device

```bash
deskbrid bluetooth pair 00:11:22:33:44:55
```

Protocol:
```json
{"action": "bluetooth.pair", "mac": "00:11:22:33:44:55"}
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

devices = client.bluetooth_list()
for device in devices:
    print(f"{device['name']}: {'connected' if device['connected'] else 'disconnected'}")
```