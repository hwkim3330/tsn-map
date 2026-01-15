use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedPacket {
    pub id: u64,
    pub timestamp: DateTime<Utc>,
    pub length: u32,
    pub data: Vec<u8>,
    pub info: PacketInfo,
    pub tsn_info: Option<TsnInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketInfo {
    pub src_mac: String,
    pub dst_mac: String,
    pub ethertype: u16,
    pub ethertype_name: String,
    pub vlan_id: Option<u16>,
    pub vlan_pcp: Option<u8>,
    pub src_ip: Option<String>,
    pub dst_ip: Option<String>,
    pub protocol: Option<String>,
    pub src_port: Option<u16>,
    pub dst_port: Option<u16>,
    pub is_ptp: bool,
    pub is_tsn: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsnInfo {
    pub stream_id: Option<String>,
    pub sequence_number: Option<u32>,
    pub traffic_class: Option<u8>,
    pub priority: Option<u8>,
    pub tsn_type: TsnType,
    pub ptp_info: Option<PtpInfo>,
    pub cbs_info: Option<CbsInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TsnType {
    Ptp,           // IEEE 1588 Precision Time Protocol
    Cbs,           // IEEE 802.1Qav Credit-Based Shaper
    Tas,           // IEEE 802.1Qbv Time-Aware Shaper
    Frer,          // IEEE 802.1CB Frame Replication and Elimination
    Srp,           // IEEE 802.1Qat Stream Reservation Protocol
    Standard,      // Regular Ethernet frame
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtpInfo {
    pub message_type: String,
    pub version: u8,
    pub domain: u8,
    pub sequence_id: u16,
    pub source_port_identity: String,
    pub correction_field: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CbsInfo {
    pub idle_slope: Option<u32>,
    pub send_slope: Option<i32>,
    pub hi_credit: Option<i32>,
    pub lo_credit: Option<i32>,
    pub traffic_class: u8,
}

impl CapturedPacket {
    pub fn from_raw(id: u64, data: &[u8], timestamp: DateTime<Utc>) -> Self {
        let info = Self::parse_packet_info(data);
        let tsn_info = Self::detect_tsn_info(data, &info);

        Self {
            id,
            timestamp,
            length: data.len() as u32,
            data: data.to_vec(),
            info,
            tsn_info,
        }
    }

    fn parse_packet_info(data: &[u8]) -> PacketInfo {
        let mut info = PacketInfo {
            src_mac: String::new(),
            dst_mac: String::new(),
            ethertype: 0,
            ethertype_name: String::from("Unknown"),
            vlan_id: None,
            vlan_pcp: None,
            src_ip: None,
            dst_ip: None,
            protocol: None,
            src_port: None,
            dst_port: None,
            is_ptp: false,
            is_tsn: false,
        };

        if data.len() < 14 {
            return info;
        }

        // Parse MAC addresses
        info.dst_mac = format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            data[0], data[1], data[2], data[3], data[4], data[5]
        );
        info.src_mac = format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            data[6], data[7], data[8], data[9], data[10], data[11]
        );

        // Parse EtherType (handle VLAN tags)
        let mut offset = 12;
        let mut ethertype = u16::from_be_bytes([data[offset], data[offset + 1]]);

        // Check for VLAN tag (0x8100)
        if ethertype == 0x8100 && data.len() >= 18 {
            let tci = u16::from_be_bytes([data[14], data[15]]);
            info.vlan_id = Some(tci & 0x0FFF);
            info.vlan_pcp = Some((tci >> 13) as u8);
            offset = 16;
            ethertype = u16::from_be_bytes([data[offset], data[offset + 1]]);
        }

        info.ethertype = ethertype;
        info.ethertype_name = match ethertype {
            0x0800 => "IPv4".to_string(),
            0x0806 => "ARP".to_string(),
            0x86DD => "IPv6".to_string(),
            0x8100 => "VLAN".to_string(),
            0x88F7 => "PTP".to_string(),
            0x22F0 => "802.1Qat SRP".to_string(),
            0x88B8 => "GOOSE".to_string(),
            0x88BA => "SV".to_string(),
            0x88CC => "LLDP".to_string(),
            0x88E5 => "802.1AE MACsec".to_string(),
            0x893A => "IEEE 1905".to_string(),
            _ => format!("0x{:04X}", ethertype),
        };

        // Check for PTP
        info.is_ptp = ethertype == 0x88F7 || Self::is_ptp_udp(data, offset + 2);

        // Parse IP layer manually
        let ip_offset = offset + 2;
        if ethertype == 0x0800 && data.len() >= ip_offset + 20 {
            // IPv4
            info.src_ip = Some(format!("{}.{}.{}.{}",
                data[ip_offset + 12], data[ip_offset + 13],
                data[ip_offset + 14], data[ip_offset + 15]));
            info.dst_ip = Some(format!("{}.{}.{}.{}",
                data[ip_offset + 16], data[ip_offset + 17],
                data[ip_offset + 18], data[ip_offset + 19]));

            let protocol = data[ip_offset + 9];
            info.protocol = Some(match protocol {
                1 => "ICMP".to_string(),
                6 => "TCP".to_string(),
                17 => "UDP".to_string(),
                _ => format!("{}", protocol),
            });

            // Parse transport layer
            let ihl = (data[ip_offset] & 0x0F) as usize * 4;
            let transport_offset = ip_offset + ihl;

            if data.len() >= transport_offset + 4 {
                info.src_port = Some(u16::from_be_bytes([data[transport_offset], data[transport_offset + 1]]));
                info.dst_port = Some(u16::from_be_bytes([data[transport_offset + 2], data[transport_offset + 3]]));

                // Check for PTP over UDP
                if protocol == 17 && (info.dst_port == Some(319) || info.dst_port == Some(320)) {
                    info.is_ptp = true;
                }
            }
        } else if ethertype == 0x86DD && data.len() >= ip_offset + 40 {
            // IPv6
            let src = &data[ip_offset + 8..ip_offset + 24];
            let dst = &data[ip_offset + 24..ip_offset + 40];
            info.src_ip = Some(format_ipv6(src));
            info.dst_ip = Some(format_ipv6(dst));

            let next_header = data[ip_offset + 6];
            info.protocol = Some(match next_header {
                6 => "TCP".to_string(),
                17 => "UDP".to_string(),
                58 => "ICMPv6".to_string(),
                _ => format!("{}", next_header),
            });
        }

        // Check if TSN-related
        info.is_tsn = info.is_ptp || info.vlan_pcp.is_some() || ethertype == 0x22F0;

        info
    }

    fn is_ptp_udp(data: &[u8], ip_offset: usize) -> bool {
        // Check if UDP ports 319 or 320 (PTP)
        if data.len() < ip_offset + 28 {
            return false;
        }

        // Check IP protocol is UDP (17)
        if data.len() > ip_offset + 9 && data[ip_offset + 9] == 17 {
            let ihl = (data[ip_offset] & 0x0F) as usize * 4;
            let udp_offset = ip_offset + ihl;
            if data.len() >= udp_offset + 4 {
                let dst_port = u16::from_be_bytes([data[udp_offset + 2], data[udp_offset + 3]]);
                return dst_port == 319 || dst_port == 320;
            }
        }
        false
    }

    fn detect_tsn_info(data: &[u8], info: &PacketInfo) -> Option<TsnInfo> {
        if !info.is_tsn && info.vlan_pcp.is_none() && !info.is_ptp {
            return None;
        }

        let mut tsn_info = TsnInfo {
            stream_id: None,
            sequence_number: None,
            traffic_class: info.vlan_pcp,
            priority: info.vlan_pcp,
            tsn_type: TsnType::Standard,
            ptp_info: None,
            cbs_info: None,
        };

        // Generate stream ID from MAC + VLAN
        if let Some(vlan) = info.vlan_id {
            tsn_info.stream_id = Some(format!("{}:{}", info.src_mac, vlan));
        }

        // Parse PTP info if PTP packet
        if info.is_ptp {
            tsn_info.tsn_type = TsnType::Ptp;
            tsn_info.ptp_info = Self::parse_ptp_info(data, info);
        }

        // Determine TSN type based on priority
        if let Some(pcp) = info.vlan_pcp {
            match pcp {
                6 | 7 => {
                    // High priority - likely CBS or scheduled traffic
                    if !info.is_ptp {
                        tsn_info.tsn_type = TsnType::Cbs;
                    }
                }
                4 | 5 => {
                    // Medium priority
                    tsn_info.tsn_type = TsnType::Cbs;
                }
                _ => {}
            }
        }

        Some(tsn_info)
    }

    fn parse_ptp_info(data: &[u8], info: &PacketInfo) -> Option<PtpInfo> {
        // Find PTP header offset
        let ptp_offset = if info.ethertype == 0x88F7 {
            if info.vlan_id.is_some() { 18 } else { 14 }
        } else if info.dst_port == Some(319) || info.dst_port == Some(320) {
            // PTP over UDP - header after UDP header
            if info.vlan_id.is_some() { 46 } else { 42 }
        } else {
            return None;
        };

        if data.len() < ptp_offset + 34 {
            return None;
        }

        let msg_type = data[ptp_offset] & 0x0F;
        let version = data[ptp_offset + 1] & 0x0F;
        let domain = data[ptp_offset + 4];
        let sequence_id = u16::from_be_bytes([data[ptp_offset + 30], data[ptp_offset + 31]]);

        let correction_bytes: [u8; 8] = data[ptp_offset + 8..ptp_offset + 16].try_into().ok()?;
        let correction = i64::from_be_bytes(correction_bytes);

        let source_port = format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}-{}",
            data[ptp_offset + 20], data[ptp_offset + 21], data[ptp_offset + 22], data[ptp_offset + 23],
            data[ptp_offset + 24], data[ptp_offset + 25], data[ptp_offset + 26], data[ptp_offset + 27],
            u16::from_be_bytes([data[ptp_offset + 28], data[ptp_offset + 29]])
        );

        let message_type = match msg_type {
            0 => "Sync",
            1 => "Delay_Req",
            2 => "Pdelay_Req",
            3 => "Pdelay_Resp",
            8 => "Follow_Up",
            9 => "Delay_Resp",
            10 => "Pdelay_Resp_Follow_Up",
            11 => "Announce",
            12 => "Signaling",
            13 => "Management",
            _ => "Unknown",
        }.to_string();

        Some(PtpInfo {
            message_type,
            version,
            domain,
            sequence_id,
            source_port_identity: source_port,
            correction_field: correction,
        })
    }
}

fn format_ipv6(bytes: &[u8]) -> String {
    let parts: Vec<String> = bytes.chunks(2)
        .map(|c| format!("{:02x}{:02x}", c[0], c[1]))
        .collect();
    parts.join(":")
}
