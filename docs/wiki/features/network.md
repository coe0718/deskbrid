# Network

Query and manage network interfaces, Wi-Fi connections, hotspots, DNS, VPNs,
and WWAN (mobile broadband).

## Actions

### network.status

Return overall network status — connectivity state and primary interface.

```bash
deskbrid network.status
```

No parameters.

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": {
    "connectivity": "full",
    "primary_interface": "wlp2s0",
    "primary_ip": "192.168.1.42",
    "gateway": "192.168.1.1"
  }
}
```

### network.interfaces

List all network interfaces with their type, IP, and state.

```bash
deskbrid network.interfaces
```

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {"name": "lo", "type": "loopback", "state": "up", "ip": "127.0.0.1"},
    {"name": "wlp2s0", "type": "wifi", "state": "up", "ip": "192.168.1.42"},
    {"name": "enp3s0", "type": "ethernet", "state": "down", "ip": null}
  ]
}
```

### network.wifi_scan

Scan for available Wi-Fi networks.

```bash
deskbrid network.wifi_scan
```

No parameters.

Response:

```json
{
  "type": "response",
  "status": "ok",
  "data": [
    {"ssid": "Home Network", "signal": 85, "secured": true, "frequency": 5180},
    {"ssid": "Guest WiFi", "signal": 60, "secured": false, "frequency": 2437}
  ]
}
```

### network.wifi_connect

Connect to a Wi-Fi network.

| Parameter | Type    | Description               |
|-----------|---------|---------------------------|
| `ssid`    | string  | Network SSID              |
| `password`| string? | WPA/WPA2 passphrase       |

```bash
deskbrid network.wifi_connect '{"ssid": "Home Network", "password": "mysecret"}'
```

```json
{
  "type": "network.wifi_connect",
  "ssid": "Home Network",
  "password": "mysecret"
}
```

### network.connection_list

List active network connections with their details.

```bash
deskbrid network.connection_list
```

No parameters.

### network.connection_profiles

List saved (configured) network connection profiles.

```bash
deskbrid network.connection_profiles
```

No parameters.

### network.create_hotspot

Create a Wi-Fi hotspot.

| Parameter | Type    | Description           |
|-----------|---------|-----------------------|
| `ssid`    | string  | Hotspot SSID          |
| `password`| string? | WPA2 passphrase       |

```bash
deskbrid network.create_hotspot '{"ssid": "deskbrid-hotspot", "password": "temp1234"}'
```

```json
{
  "type": "network.create_hotspot",
  "ssid": "deskbrid-hotspot",
  "password": "temp1234"
}
```

### network.stop_hotspot

Stop the currently active hotspot.

```bash
deskbrid network.stop_hotspot
```

No parameters.

### network.wifi_enable

Enable or disable the Wi-Fi radio.

| Parameter | Type | Description          |
|-----------|------|----------------------|
| `enabled` | bool | `true` to enable Wi-Fi |

```bash
deskbrid network.wifi_enable '{"enabled": true}'
```

### network.wwan_enable

Enable or disable the WWAN (mobile broadband) radio.

| Parameter | Type | Description              |
|-----------|------|--------------------------|
| `enabled` | bool | `true` to enable WWAN    |

```bash
deskbrid network.wwan_enable '{"enabled": false}'
```

### network.dns_set

Set custom DNS servers for the active connection.

| Parameter | Type       | Description              |
|-----------|------------|--------------------------|
| `dns`     | string[]   | List of DNS server IPs   |

```bash
deskbrid network.dns_set '{"dns": ["1.1.1.1", "8.8.8.8"]}'
```

### network.dns_reset

Reset DNS to automatic (DHCP) configuration.

```bash
deskbrid network.dns_reset
```

### network.vpn_connect

Connect to a VPN by profile name.

| Parameter      | Type   | Description            |
|----------------|--------|------------------------|
| `profile_name` | string | VPN profile name       |

```bash
deskbrid network.vpn_connect '{"profile_name": "Work VPN"}'
```

### network.vpn_disconnect

Disconnect the active VPN connection.

```bash
deskbrid network.vpn_disconnect
```

## Python Example

```python
from deskbrid import Deskbrid

client = Deskbrid()

# Check status
status = client.network_status()
print(f"Connected: {status['connectivity']}")

# Scan Wi-Fi
networks = client.network_wifi_scan()
strongest = max(networks, key=lambda n: n["signal"])
print(f"Strongest network: {strongest['ssid']} ({strongest['signal']}%)")

# Connect
client.network_wifi_connect(ssid=strongest["ssid"], password="mysecret")

# Custom DNS
client.network_dns_set(dns=["1.1.1.1", "1.0.0.1"])
```

## Requirements

- NetworkManager (`nmcli` / D-Bus API) is required for all network operations.
- `systemd-resolved` for DNS operations (or NetworkManager's built-in DNS).
- VPN profiles must be pre-configured in NetworkManager.
- Hotspots require a Wi-Fi adapter that supports AP mode.

## Current Status

**Stable** — network status, interfaces, Wi-Fi scan/connect.
**Experimental** — hotspots, WWAN, DNS, VPN.
