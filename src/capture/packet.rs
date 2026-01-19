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
    // TCP specific
    pub tcp_flags: Option<TcpFlags>,
    pub seq_num: Option<u32>,
    pub ack_num: Option<u32>,
    pub window_size: Option<u16>,
    // ICMP specific
    pub icmp_type: Option<u8>,
    pub icmp_code: Option<u8>,
    // ARP specific
    pub arp_op: Option<u16>,
    // IP specific
    pub ttl: Option<u8>,
    pub ip_id: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpFlags {
    pub fin: bool,
    pub syn: bool,
    pub rst: bool,
    pub psh: bool,
    pub ack: bool,
    pub urg: bool,
    pub ece: bool,
    pub cwr: bool,
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
            tcp_flags: None,
            seq_num: None,
            ack_num: None,
            window_size: None,
            icmp_type: None,
            icmp_code: None,
            arp_op: None,
            ttl: None,
            ip_id: None,
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

        // Parse EtherType or Length (IEEE 802.3 vs Ethernet II)
        let mut offset = 12;
        let mut eth_type_or_len = u16::from_be_bytes([data[offset], data[offset + 1]]);

        // Check for VLAN tag (0x8100)
        if eth_type_or_len == 0x8100 && data.len() >= 18 {
            let tci = u16::from_be_bytes([data[14], data[15]]);
            info.vlan_id = Some(tci & 0x0FFF);
            info.vlan_pcp = Some((tci >> 13) as u8);
            offset = 16;
            eth_type_or_len = u16::from_be_bytes([data[offset], data[offset + 1]]);
        }

        // IEEE 802.3 vs Ethernet II detection
        // If value <= 1500 (0x05DC), it's a Length field (IEEE 802.3)
        // If value >= 1536 (0x0600), it's an EtherType (Ethernet II)
        let ethertype = if eth_type_or_len <= 1500 {
            // IEEE 802.3 frame with LLC header
            // LLC header: DSAP (1) + SSAP (1) + Control (1-2)
            let llc_offset = offset + 2;
            if data.len() >= llc_offset + 3 {
                let dsap = data[llc_offset];
                let ssap = data[llc_offset + 1];

                // SNAP: DSAP=0xAA, SSAP=0xAA
                if dsap == 0xAA && ssap == 0xAA && data.len() >= llc_offset + 8 {
                    // SNAP header: OUI (3) + EtherType (2)
                    u16::from_be_bytes([data[llc_offset + 6], data[llc_offset + 7]])
                } else if dsap == 0x42 && ssap == 0x42 {
                    // STP (Spanning Tree Protocol)
                    0x0026  // Use custom marker for STP
                } else if dsap == 0xFE && ssap == 0xFE {
                    // OSI protocols
                    0x00FE
                } else {
                    // Generic LLC - mark as 802.3
                    0x0001  // Custom marker for raw 802.3
                }
            } else {
                0x0001
            }
        } else {
            eth_type_or_len
        };

        info.ethertype = ethertype;
        info.ethertype_name = match ethertype {
            0x0001 => "802.3".to_string(),       // Raw IEEE 802.3 LLC
            0x0026 => "STP".to_string(),         // Spanning Tree Protocol
            0x0800 => "IPv4".to_string(),
            0x0806 => "ARP".to_string(),
            0x86DD => "IPv6".to_string(),
            0x8100 => "VLAN".to_string(),
            0x88A8 => "QinQ".to_string(),          // 802.1ad Provider Bridging
            0x88F7 => "PTP".to_string(),
            0x22F0 => "802.1Qat SRP".to_string(),
            0x88B8 => "GOOSE".to_string(),
            0x88BA => "SV".to_string(),
            0x88CC => "LLDP".to_string(),
            0x88E5 => "MACsec".to_string(),        // 802.1AE
            0x893A => "IEEE 1905".to_string(),
            0x8899 => "RRCP".to_string(),          // Realtek Remote Control Protocol
            0x9000 => "Loopback".to_string(),      // Configuration Test Protocol (loop detection)
            0x0842 => "WoL".to_string(),           // Wake-on-LAN
            0x8035 => "RARP".to_string(),          // Reverse ARP
            0x809B => "AppleTalk".to_string(),
            0x80F3 => "AARP".to_string(),          // AppleTalk ARP
            0x8137 => "IPX".to_string(),
            0x8863 => "PPPoE-D".to_string(),       // PPPoE Discovery
            0x8864 => "PPPoE-S".to_string(),       // PPPoE Session
            0x88E1 => "HomePlug".to_string(),
            0x8902 => "CFM".to_string(),           // 802.1ag Connectivity Fault Management
            0x22EA => "SRP".to_string(),           // Stream Reservation Protocol
            0x2000 => "CDP".to_string(),           // Cisco Discovery Protocol
            0x2004 => "CGMP".to_string(),          // Cisco Group Management Protocol
            0x887B => "HomePlug".to_string(),      // HomePlug GP
            0x887E => "MVRP".to_string(),          // Multiple VLAN Registration Protocol (802.1ak)
            0x8880 => "MRP".to_string(),           // Multiple Registration Protocol
            _ => format!("0x{:04X}", ethertype),
        };

        // Check for PTP
        info.is_ptp = ethertype == 0x88F7 || Self::is_ptp_udp(data, offset + 2);

        // Parse ARP
        let ip_offset = offset + 2;
        if ethertype == 0x0806 && data.len() >= ip_offset + 8 {
            // ARP operation: offset 6-7 in ARP header
            info.arp_op = Some(u16::from_be_bytes([data[ip_offset + 6], data[ip_offset + 7]]));
            // ARP sender IP (offset 14) and target IP (offset 24)
            if data.len() >= ip_offset + 28 {
                info.src_ip = Some(format!("{}.{}.{}.{}",
                    data[ip_offset + 14], data[ip_offset + 15],
                    data[ip_offset + 16], data[ip_offset + 17]));
                info.dst_ip = Some(format!("{}.{}.{}.{}",
                    data[ip_offset + 24], data[ip_offset + 25],
                    data[ip_offset + 26], data[ip_offset + 27]));
            }
        }

        // Parse IP layer
        if ethertype == 0x0800 && data.len() >= ip_offset + 20 {
            // IPv4 header fields
            info.ttl = Some(data[ip_offset + 8]);
            info.ip_id = Some(u16::from_be_bytes([data[ip_offset + 4], data[ip_offset + 5]]));

            info.src_ip = Some(format!("{}.{}.{}.{}",
                data[ip_offset + 12], data[ip_offset + 13],
                data[ip_offset + 14], data[ip_offset + 15]));
            info.dst_ip = Some(format!("{}.{}.{}.{}",
                data[ip_offset + 16], data[ip_offset + 17],
                data[ip_offset + 18], data[ip_offset + 19]));

            let protocol = data[ip_offset + 9];
            info.protocol = Some(match protocol {
                0 => "HOPOPT".to_string(),
                1 => "ICMP".to_string(),
                2 => "IGMP".to_string(),
                4 => "IP-in-IP".to_string(),
                6 => "TCP".to_string(),
                17 => "UDP".to_string(),
                41 => "IPv6".to_string(),
                43 => "IPv6-Route".to_string(),
                44 => "IPv6-Frag".to_string(),
                47 => "GRE".to_string(),
                50 => "ESP".to_string(),
                51 => "AH".to_string(),
                58 => "ICMPv6".to_string(),
                59 => "IPv6-NoNxt".to_string(),
                60 => "IPv6-Opts".to_string(),
                88 => "EIGRP".to_string(),
                89 => "OSPF".to_string(),
                103 => "PIM".to_string(),
                112 => "VRRP".to_string(),
                132 => "SCTP".to_string(),
                _ => format!("Proto({})", protocol),
            });

            let ihl = (data[ip_offset] & 0x0F) as usize * 4;
            let transport_offset = ip_offset + ihl;

            // Parse ICMP
            if protocol == 1 && data.len() >= transport_offset + 2 {
                info.icmp_type = Some(data[transport_offset]);
                info.icmp_code = Some(data[transport_offset + 1]);
            }

            // Parse TCP
            if protocol == 6 && data.len() >= transport_offset + 20 {
                info.src_port = Some(u16::from_be_bytes([data[transport_offset], data[transport_offset + 1]]));
                info.dst_port = Some(u16::from_be_bytes([data[transport_offset + 2], data[transport_offset + 3]]));
                info.seq_num = Some(u32::from_be_bytes([
                    data[transport_offset + 4], data[transport_offset + 5],
                    data[transport_offset + 6], data[transport_offset + 7]
                ]));
                info.ack_num = Some(u32::from_be_bytes([
                    data[transport_offset + 8], data[transport_offset + 9],
                    data[transport_offset + 10], data[transport_offset + 11]
                ]));
                info.window_size = Some(u16::from_be_bytes([data[transport_offset + 14], data[transport_offset + 15]]));

                let flags = data[transport_offset + 13];
                info.tcp_flags = Some(TcpFlags {
                    fin: (flags & 0x01) != 0,
                    syn: (flags & 0x02) != 0,
                    rst: (flags & 0x04) != 0,
                    psh: (flags & 0x08) != 0,
                    ack: (flags & 0x10) != 0,
                    urg: (flags & 0x20) != 0,
                    ece: (flags & 0x40) != 0,
                    cwr: (flags & 0x80) != 0,
                });
            }

            // Parse UDP
            if protocol == 17 && data.len() >= transport_offset + 4 {
                info.src_port = Some(u16::from_be_bytes([data[transport_offset], data[transport_offset + 1]]));
                info.dst_port = Some(u16::from_be_bytes([data[transport_offset + 2], data[transport_offset + 3]]));

                if info.dst_port == Some(319) || info.dst_port == Some(320) {
                    info.is_ptp = true;
                }
            }
        } else if ethertype == 0x86DD && data.len() >= ip_offset + 40 {
            // IPv6
            info.ttl = Some(data[ip_offset + 7]); // Hop Limit
            let src = &data[ip_offset + 8..ip_offset + 24];
            let dst = &data[ip_offset + 24..ip_offset + 40];
            info.src_ip = Some(format_ipv6(src));
            info.dst_ip = Some(format_ipv6(dst));

            let next_header = data[ip_offset + 6];
            info.protocol = Some(match next_header {
                0 => "HOPOPT".to_string(),
                6 => "TCP".to_string(),
                17 => "UDP".to_string(),
                43 => "IPv6-Route".to_string(),
                44 => "IPv6-Frag".to_string(),
                50 => "ESP".to_string(),
                51 => "AH".to_string(),
                58 => "ICMPv6".to_string(),
                59 => "IPv6-NoNxt".to_string(),
                60 => "IPv6-Opts".to_string(),
                _ => format!("Proto({})", next_header),
            });

            // IPv6 transport layer (fixed 40 byte header)
            let transport_offset = ip_offset + 40;

            // Parse ICMPv6
            if next_header == 58 && data.len() >= transport_offset + 2 {
                info.icmp_type = Some(data[transport_offset]);
                info.icmp_code = Some(data[transport_offset + 1]);
            }

            // Parse TCP
            if next_header == 6 && data.len() >= transport_offset + 20 {
                info.src_port = Some(u16::from_be_bytes([data[transport_offset], data[transport_offset + 1]]));
                info.dst_port = Some(u16::from_be_bytes([data[transport_offset + 2], data[transport_offset + 3]]));
                info.seq_num = Some(u32::from_be_bytes([
                    data[transport_offset + 4], data[transport_offset + 5],
                    data[transport_offset + 6], data[transport_offset + 7]
                ]));
                info.ack_num = Some(u32::from_be_bytes([
                    data[transport_offset + 8], data[transport_offset + 9],
                    data[transport_offset + 10], data[transport_offset + 11]
                ]));
                let flags = data[transport_offset + 13];
                info.tcp_flags = Some(TcpFlags {
                    fin: (flags & 0x01) != 0,
                    syn: (flags & 0x02) != 0,
                    rst: (flags & 0x04) != 0,
                    psh: (flags & 0x08) != 0,
                    ack: (flags & 0x10) != 0,
                    urg: (flags & 0x20) != 0,
                    ece: (flags & 0x40) != 0,
                    cwr: (flags & 0x80) != 0,
                });
            }

            // Parse UDP
            if next_header == 17 && data.len() >= transport_offset + 4 {
                info.src_port = Some(u16::from_be_bytes([data[transport_offset], data[transport_offset + 1]]));
                info.dst_port = Some(u16::from_be_bytes([data[transport_offset + 2], data[transport_offset + 3]]));
            }
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
