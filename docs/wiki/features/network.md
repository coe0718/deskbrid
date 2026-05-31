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
{"type": "network.status"}
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
{"type": "network.wifi"}
```

## Network Connections

```bash
deskbrid network connections
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {
      "name": "MyHomeWiFi",
      "type": "wifi",
      "device": "wlan0",
      "state": "activated",
      "ip4": "192.168.1.100",
      "ip6": "fe80::abcd:ef01:2345:6789"
    },
    {
      "name": "Ethernet",
      "type": "ethernet",
      "device": "eth0",
      "state": "deactivated"
    }
  ]
}
```

Protocol:
```json
{"type": "network.connections.list"}
```

## Connection Profiles

```bash
deskbrid network profiles
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {
      "name": "MyHomeWiFi",
      "type": "wifi",
      "autoconnect": true
    },
    {
      "name": "WorkVPN",
      "type": "vpn",
      "autoconnect": false
    }
  ]
}
```

Protocol:
```json
{"type": "network.connections.profiles"}
```

## Start Hotspot

```bash
deskbrid network hotspot start --ssid MyHotspot --password secret123
```

Protocol:
```json
{"type": "network.hotspot.start", "ssid": "MyHotspot", "password": "secret123"}
```

## Stop Hotspot

```bash
deskbrid network hotspot stop
```

Protocol:
```json
{"type": "network.hotspot.stop"}
```

## Enable/Disable WiFi

```bash
deskbrid network wifi enable
deskbrid network wifi disable
```

Protocol:
```json
{"type": "network.wifi.enable", "enabled": true}
```
```json
{"type": "network.wifi.enable", "enabled": false}
```

## Set DNS

```bash
deskbrid network dns set --dns 8.8.8.8 --dns 8.8.4.4
```

Protocol:
```json
{"type": "network.dns.set", "dns": ["8.8.8.8", "8.8.4.4"]}
```

## Reset DNS

```bash
deskbrid network dns reset
```

Protocol:
```json
{"type": "network.dns.reset"}
```

## VPN Actions

```bash
deskbrid network vpn connect --profile WorkVPN
deskbrid network vpn disconnect --profile WorkVPN
```

Protocol:
```json
{"type": "network.vpn.connect", "profile_name": "WorkVPN"}
```
```json
{"type": "network.vpn.disconnect", "profile_name": "WorkVPN"}
```

## WWAN Enable/Disable

```bash
deskbrid network wwan enable
deskbrid network wwan disable
```

Protocol:
```json
{"type": "network.wwan.enable", "enabled": true}
```
```json
{"type": "network.wwan.enable", "enabled": false}
```

## TCP Mode

Deskbrid can also listen on a TCP port for remote connections (requires `--tcp-port` and `--tcp-token` flags when starting the daemon).

```bash
deskbrid network tcp info
```

Response:
```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "listening": true,
    "port": 18796,
    "token_set": true
  }
}
```

Protocol:
```json
{"type": "network.tcp.info"}
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

wifi = client.network_wifi()
for network in wifi:
    print(f"{network['ssid']}: {network['signal']}%")

connections = client.network_connections_list()
for conn in connections:
    print(f"{conn['name']} ({conn['type']}): {conn['state']}")

profiles = client.network_connections_profiles()
for profile in profiles:
    print(f"{profile['name']} ({profile['type']}): autoconnect={profile['autoconnect']}")
```