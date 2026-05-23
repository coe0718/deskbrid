# Network

Query network status and WiFi information.

## Network Status

```bash
deskbrid network status
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "connected": true,
    "interface": "wlan0",
    "type": "wifi",
    "ssid": "MyNetwork",
    "signal": 85
  }
}
```

Protocol:
```json
{"action": "network.status"}
```

## WiFi Networks

```bash
deskbrid network wifi
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {"ssid": "MyNetwork", "signal": 85, "secured": true},
    {"ssid": "GuestNetwork", "signal": 42, "secured": false}
  ]
}
```

Protocol:
```json
{"action": "network.wifi"}
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

status = client.network_status()
if status["connected"]:
    print(f"Connected to {status['ssid']}")
else:
    print("No network connection")
```