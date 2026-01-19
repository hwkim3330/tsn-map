//! LLDP (Link Layer Discovery Protocol) parser
//! IEEE 802.1AB - Station and Media Access Control Connectivity Discovery

use super::{LldpInfo, DeviceCapability};

// LLDP TLV types
const TLV_END: u8 = 0;
const TLV_CHASSIS_ID: u8 = 1;
const TLV_PORT_ID: u8 = 2;
const TLV_TTL: u8 = 3;
const TLV_PORT_DESCRIPTION: u8 = 4;
const TLV_SYSTEM_NAME: u8 = 5;
const TLV_SYSTEM_DESCRIPTION: u8 = 6;
const TLV_SYSTEM_CAPABILITIES: u8 = 7;
const TLV_MANAGEMENT_ADDRESS: u8 = 8;
const TLV_ORGANIZATION_SPECIFIC: u8 = 127;

// Chassis ID subtypes
const CHASSIS_SUBTYPE_CHASSIS_COMPONENT: u8 = 1;
const CHASSIS_SUBTYPE_INTERFACE_ALIAS: u8 = 2;
const CHASSIS_SUBTYPE_PORT_COMPONENT: u8 = 3;
const CHASSIS_SUBTYPE_MAC_ADDRESS: u8 = 4;
const CHASSIS_SUBTYPE_NETWORK_ADDRESS: u8 = 5;
const CHASSIS_SUBTYPE_INTERFACE_NAME: u8 = 6;
const CHASSIS_SUBTYPE_LOCALLY_ASSIGNED: u8 = 7;

// Port ID subtypes
const PORT_SUBTYPE_INTERFACE_ALIAS: u8 = 1;
const PORT_SUBTYPE_PORT_COMPONENT: u8 = 2;
const PORT_SUBTYPE_MAC_ADDRESS: u8 = 3;
const PORT_SUBTYPE_NETWORK_ADDRESS: u8 = 4;
const PORT_SUBTYPE_INTERFACE_NAME: u8 = 5;
const PORT_SUBTYPE_AGENT_CIRCUIT_ID: u8 = 6;
const PORT_SUBTYPE_LOCALLY_ASSIGNED: u8 = 7;

// System capabilities bits
const CAP_OTHER: u16 = 0x0001;
const CAP_REPEATER: u16 = 0x0002;
const CAP_BRIDGE: u16 = 0x0004;
const CAP_WLAN_AP: u16 = 0x0008;
const CAP_ROUTER: u16 = 0x0010;
const CAP_TELEPHONE: u16 = 0x0020;
const CAP_DOCSIS_CABLE: u16 = 0x0040;
const CAP_STATION_ONLY: u16 = 0x0080;
const CAP_CVLAN: u16 = 0x0100;
const CAP_SVLAN: u16 = 0x0200;
const CAP_TWO_PORT_MAC: u16 = 0x0400;

/// Parse LLDP packet data
pub fn parse_lldp_packet(data: &[u8]) -> Option<LldpInfo> {
    // LLDP starts after Ethernet header (14 bytes for standard, more for VLAN)
    let mut offset = 14;

    // Check for VLAN tag (0x8100)
    if data.len() > 16 && data[12] == 0x81 && data[13] == 0x00 {
        offset = 18; // Skip VLAN tag
    }

    // Verify LLDP ethertype (0x88CC)
    if data.len() < offset + 2 {
        return None;
    }

    let ethertype = u16::from_be_bytes([data[offset - 2], data[offset - 1]]);
    if ethertype != 0x88CC {
        // Try without VLAN
        if data.len() > 14 && data[12] == 0x88 && data[13] == 0xCC {
            offset = 14;
        } else {
            return None;
        }
    }

    let mut info = LldpInfo {
        chassis_id: String::new(),
        chassis_id_subtype: 0,
        port_id: String::new(),
        port_id_subtype: 0,
        port_description: None,
        system_name: None,
        system_description: None,
        system_capabilities: Vec::new(),
        enabled_capabilities: Vec::new(),
        management_addresses: Vec::new(),
    };

    // Parse TLVs
    while offset < data.len() - 1 {
        let tlv_header = u16::from_be_bytes([data[offset], data[offset + 1]]);
        let tlv_type = (tlv_header >> 9) as u8;
        let tlv_length = (tlv_header & 0x01FF) as usize;

        offset += 2;

        if offset + tlv_length > data.len() {
            break;
        }

        let tlv_data = &data[offset..offset + tlv_length];

        match tlv_type {
            TLV_END => break,

            TLV_CHASSIS_ID => {
                if tlv_length >= 1 {
                    info.chassis_id_subtype = tlv_data[0];
                    info.chassis_id = parse_id(&tlv_data[1..], info.chassis_id_subtype);
                }
            }

            TLV_PORT_ID => {
                if tlv_length >= 1 {
                    info.port_id_subtype = tlv_data[0];
                    info.port_id = parse_id(&tlv_data[1..], info.port_id_subtype);
                }
            }

            TLV_TTL => {
                // TTL in seconds - skip for now
            }

            TLV_PORT_DESCRIPTION => {
                info.port_description = Some(String::from_utf8_lossy(tlv_data).trim().to_string());
            }

            TLV_SYSTEM_NAME => {
                info.system_name = Some(String::from_utf8_lossy(tlv_data).trim().to_string());
            }

            TLV_SYSTEM_DESCRIPTION => {
                info.system_description = Some(String::from_utf8_lossy(tlv_data).trim().to_string());
            }

            TLV_SYSTEM_CAPABILITIES => {
                if tlv_length >= 4 {
                    let system_caps = u16::from_be_bytes([tlv_data[0], tlv_data[1]]);
                    let enabled_caps = u16::from_be_bytes([tlv_data[2], tlv_data[3]]);

                    info.system_capabilities = parse_capabilities(system_caps);
                    info.enabled_capabilities = parse_capabilities(enabled_caps);
                }
            }

            TLV_MANAGEMENT_ADDRESS => {
                if let Some(addr) = parse_management_address(tlv_data) {
                    info.management_addresses.push(addr);
                }
            }

            TLV_ORGANIZATION_SPECIFIC => {
                // Could parse IEEE 802.1, 802.3, or vendor-specific TLVs
                // For now, skip
            }

            _ => {
                // Unknown TLV, skip
            }
        }

        offset += tlv_length;
    }

    // Validate that we got required TLVs
    if info.chassis_id.is_empty() || info.port_id.is_empty() {
        return None;
    }

    Some(info)
}

/// Parse chassis/port ID based on subtype
fn parse_id(data: &[u8], subtype: u8) -> String {
    match subtype {
        // MAC address (chassis subtype 4, port subtype 3)
        3 | 4 => {
            if data.len() >= 6 {
                format!(
                    "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                    data[0], data[1], data[2], data[3], data[4], data[5]
                )
            } else {
                hex::encode(data)
            }
        }
        // Network address (chassis subtype 5, port subtype 4 - but 4 already covered)
        5 => {
            parse_network_address(data)
        }
        // Interface name/alias, locally assigned (subtypes 1,2,6,7)
        1 | 2 | 6 | 7 => {
            String::from_utf8_lossy(data).trim().to_string()
        }
        _ => {
            // Default: try UTF-8, fallback to hex
            let s = String::from_utf8_lossy(data).trim().to_string();
            if s.chars().all(|c| c.is_ascii_graphic() || c.is_ascii_whitespace()) {
                s
            } else {
                hex::encode(data)
            }
        }
    }
}

/// Parse network address (IANA address family)
fn parse_network_address(data: &[u8]) -> String {
    if data.is_empty() {
        return String::new();
    }

    let addr_family = data[0];
    let addr_data = &data[1..];

    match addr_family {
        1 => {
            // IPv4
            if addr_data.len() >= 4 {
                format!("{}.{}.{}.{}", addr_data[0], addr_data[1], addr_data[2], addr_data[3])
            } else {
                hex::encode(addr_data)
            }
        }
        2 => {
            // IPv6
            if addr_data.len() >= 16 {
                let mut parts = Vec::new();
                for i in 0..8 {
                    let part = u16::from_be_bytes([addr_data[i * 2], addr_data[i * 2 + 1]]);
                    parts.push(format!("{:x}", part));
                }
                parts.join(":")
            } else {
                hex::encode(addr_data)
            }
        }
        6 => {
            // MAC address
            if addr_data.len() >= 6 {
                format!(
                    "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                    addr_data[0], addr_data[1], addr_data[2],
                    addr_data[3], addr_data[4], addr_data[5]
                )
            } else {
                hex::encode(addr_data)
            }
        }
        _ => hex::encode(addr_data),
    }
}

/// Parse management address TLV
fn parse_management_address(data: &[u8]) -> Option<String> {
    if data.len() < 2 {
        return None;
    }

    let addr_len = data[0] as usize;
    if data.len() < 1 + addr_len {
        return None;
    }

    let addr_data = &data[1..1 + addr_len];
    Some(parse_network_address(addr_data))
}

/// Parse capability bits into capability list
fn parse_capabilities(caps: u16) -> Vec<DeviceCapability> {
    let mut result = Vec::new();

    if caps & CAP_OTHER != 0 { result.push(DeviceCapability::Other); }
    if caps & CAP_REPEATER != 0 { result.push(DeviceCapability::Repeater); }
    if caps & CAP_BRIDGE != 0 { result.push(DeviceCapability::Bridge); }
    if caps & CAP_WLAN_AP != 0 { result.push(DeviceCapability::WlanAP); }
    if caps & CAP_ROUTER != 0 { result.push(DeviceCapability::Router); }
    if caps & CAP_TELEPHONE != 0 { result.push(DeviceCapability::Telephone); }
    if caps & CAP_DOCSIS_CABLE != 0 { result.push(DeviceCapability::DocsisCableDevice); }
    if caps & CAP_STATION_ONLY != 0 { result.push(DeviceCapability::StationOnly); }
    if caps & CAP_CVLAN != 0 { result.push(DeviceCapability::CVlanComponent); }
    if caps & CAP_SVLAN != 0 { result.push(DeviceCapability::SVlanComponent); }
    if caps & CAP_TWO_PORT_MAC != 0 { result.push(DeviceCapability::TwoPortMacRelay); }

    result
}

/// Simple hex encoding (to avoid external dependency)
mod hex {
    pub fn encode(data: &[u8]) -> String {
        data.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_capabilities() {
        let caps = parse_capabilities(CAP_BRIDGE | CAP_ROUTER);
        assert!(caps.contains(&DeviceCapability::Bridge));
        assert!(caps.contains(&DeviceCapability::Router));
        assert!(!caps.contains(&DeviceCapability::WlanAP));
    }
}
