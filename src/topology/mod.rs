use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use crate::capture::CapturedPacket;

/// Network node representing a device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkNode {
    pub id: String,
    pub mac_address: String,
    pub ip_addresses: Vec<String>,
    pub hostname: Option<String>,
    pub node_type: NodeType,
    pub vendor: Option<String>,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub tsn_capable: bool,
    pub ptp_role: Option<PtpRole>,
    pub vlan_memberships: Vec<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeType {
    Host,
    Switch,
    Router,
    EndStation,
    TsnBridge,
    PtpGrandmaster,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PtpRole {
    Grandmaster,
    BoundaryClock,
    OrdinaryClock,
    TransparentClock,
}

/// Network link between two nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkLink {
    pub id: String,
    pub source: String,
    pub target: String,
    pub packets: u64,
    pub bytes: u64,
    pub bandwidth_mbps: f64,
    pub latency_us: Option<f64>,
    pub vlan_ids: Vec<u16>,
    pub traffic_classes: Vec<u8>,
    pub is_tsn_path: bool,
}

/// Network topology graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTopology {
    pub nodes: Vec<NetworkNode>,
    pub links: Vec<NetworkLink>,
    pub last_updated: DateTime<Utc>,
    pub scan_duration_ms: u64,
}

pub struct TopologyManager {
    nodes: HashMap<String, NetworkNode>,
    links: HashMap<String, NetworkLink>,
    mac_to_ip: HashMap<String, HashSet<String>>,
    oui_database: HashMap<String, String>,
}

impl TopologyManager {
    pub fn new() -> Self {
        let mut manager = Self {
            nodes: HashMap::new(),
            links: HashMap::new(),
            mac_to_ip: HashMap::new(),
            oui_database: HashMap::new(),
        };

        // Initialize OUI database with common TSN vendors
        manager.init_oui_database();
        manager
    }

    fn init_oui_database(&mut self) {
        // Common networking and TSN equipment vendors
        let ouis = vec![
            ("00:1a:6b", "Microchip Technology"),
            ("00:04:25", "Microchip Technology"),
            ("d8:80:39", "Microchip Technology"),
            ("00:1e:c0", "Microchip Technology"),
            ("00:60:6e", "DLOG"),
            ("00:0d:b9", "PC Engines"),
            ("00:1b:21", "Intel"),
            ("00:1f:c6", "Intel"),
            ("3c:fd:fe", "Intel"),
            ("a0:36:9f", "Intel"),
            ("00:1c:73", "Arista"),
            ("00:1d:b5", "Juniper"),
            ("00:17:cb", "Juniper"),
            ("00:1e:0b", "Hewlett Packard"),
            ("00:25:b3", "Hewlett Packard"),
            ("00:1f:29", "Hewlett Packard"),
            ("00:50:56", "VMware"),
            ("00:0c:29", "VMware"),
            ("00:15:5d", "Microsoft Hyper-V"),
            ("52:54:00", "QEMU"),
            ("08:00:27", "VirtualBox"),
            ("00:03:ff", "Microsoft"),
            ("00:00:5e", "IANA (VRRP/HSRP)"),
            ("01:00:5e", "IPv4 Multicast"),
            ("33:33:00", "IPv6 Multicast"),
            ("01:1b:19", "PTP Multicast"),
            ("01:80:c2", "IEEE 802.1 Multicast"),
        ];

        for (oui, vendor) in ouis {
            self.oui_database.insert(oui.to_string(), vendor.to_string());
        }
    }

    pub fn process_packet(&mut self, packet: &CapturedPacket) {
        // Update source node
        self.update_node(&packet.info.src_mac, packet, true);

        // Update destination node (if not multicast/broadcast)
        if !packet.info.dst_mac.starts_with("01:")
            && !packet.info.dst_mac.starts_with("33:33")
            && packet.info.dst_mac != "ff:ff:ff:ff:ff:ff"
        {
            self.update_node(&packet.info.dst_mac, packet, false);
        }

        // Update link
        self.update_link(&packet.info.src_mac, &packet.info.dst_mac, packet);
    }

    fn update_node(&mut self, mac: &str, packet: &CapturedPacket, is_source: bool) {
        // Look up vendor first to avoid borrow issues
        let vendor = self.lookup_vendor(mac);
        let node_type = self.infer_node_type(mac, &vendor);

        let node = self.nodes.entry(mac.to_string()).or_insert_with(|| {
            NetworkNode {
                id: mac.to_string(),
                mac_address: mac.to_string(),
                ip_addresses: Vec::new(),
                hostname: None,
                node_type,
                vendor,
                first_seen: packet.timestamp,
                last_seen: packet.timestamp,
                packets_sent: 0,
                packets_received: 0,
                bytes_sent: 0,
                bytes_received: 0,
                tsn_capable: false,
                ptp_role: None,
                vlan_memberships: Vec::new(),
            }
        });

        node.last_seen = packet.timestamp;

        if is_source {
            node.packets_sent += 1;
            node.bytes_sent += packet.length as u64;

            // Update IP address
            if let Some(ref ip) = packet.info.src_ip {
                if !node.ip_addresses.contains(ip) {
                    node.ip_addresses.push(ip.clone());
                }
                self.mac_to_ip
                    .entry(mac.to_string())
                    .or_default()
                    .insert(ip.clone());
            }
        } else {
            node.packets_received += 1;
            node.bytes_received += packet.length as u64;

            if let Some(ref ip) = packet.info.dst_ip {
                if !node.ip_addresses.contains(ip) {
                    node.ip_addresses.push(ip.clone());
                }
            }
        }

        // Update VLAN membership
        if let Some(vlan) = packet.info.vlan_id {
            if !node.vlan_memberships.contains(&vlan) {
                node.vlan_memberships.push(vlan);
            }
        }

        // Check TSN capability
        if packet.info.is_tsn || packet.info.is_ptp || packet.info.vlan_pcp.is_some() {
            node.tsn_capable = true;
        }

        // Update PTP role
        if packet.info.is_ptp {
            if let Some(ref tsn_info) = packet.tsn_info {
                if let Some(ref ptp_info) = tsn_info.ptp_info {
                    match ptp_info.message_type.as_str() {
                        "Announce" | "Sync" if is_source => {
                            node.ptp_role = Some(PtpRole::Grandmaster);
                            node.node_type = NodeType::PtpGrandmaster;
                        }
                        "Delay_Req" if is_source => {
                            if node.ptp_role.is_none() {
                                node.ptp_role = Some(PtpRole::OrdinaryClock);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn update_link(&mut self, src: &str, dst: &str, packet: &CapturedPacket) {
        let link_id = format!("{}:{}", src, dst);

        let link = self.links.entry(link_id.clone()).or_insert_with(|| {
            NetworkLink {
                id: link_id,
                source: src.to_string(),
                target: dst.to_string(),
                packets: 0,
                bytes: 0,
                bandwidth_mbps: 0.0,
                latency_us: None,
                vlan_ids: Vec::new(),
                traffic_classes: Vec::new(),
                is_tsn_path: false,
            }
        });

        link.packets += 1;
        link.bytes += packet.length as u64;

        // Update VLAN IDs
        if let Some(vlan) = packet.info.vlan_id {
            if !link.vlan_ids.contains(&vlan) {
                link.vlan_ids.push(vlan);
            }
        }

        // Update traffic classes
        if let Some(pcp) = packet.info.vlan_pcp {
            if !link.traffic_classes.contains(&pcp) {
                link.traffic_classes.push(pcp);
            }
        }

        // Check if TSN path
        if packet.info.is_tsn || packet.info.is_ptp {
            link.is_tsn_path = true;
        }
    }

    fn lookup_vendor(&self, mac: &str) -> Option<String> {
        let oui = mac.to_lowercase();
        if oui.len() >= 8 {
            let oui_prefix = &oui[..8];
            self.oui_database.get(oui_prefix).cloned()
        } else {
            None
        }
    }

    fn infer_node_type(&self, mac: &str, vendor: &Option<String>) -> NodeType {
        // Check for multicast addresses
        if mac.starts_with("01:1b:19") {
            return NodeType::PtpGrandmaster;
        }
        if mac.starts_with("01:80:c2") {
            return NodeType::TsnBridge;
        }
        if mac.starts_with("01:") || mac.starts_with("33:33") {
            return NodeType::Unknown;
        }

        // Infer from vendor
        if let Some(v) = vendor {
            let v_lower = v.to_lowercase();
            if v_lower.contains("microchip") {
                return NodeType::TsnBridge;
            }
            if v_lower.contains("cisco") || v_lower.contains("juniper") || v_lower.contains("arista") {
                return NodeType::Switch;
            }
        }

        NodeType::EndStation
    }

    pub fn get_topology(&self) -> NetworkTopology {
        NetworkTopology {
            nodes: self.nodes.values().cloned().collect(),
            links: self.links.values().cloned().collect(),
            last_updated: Utc::now(),
            scan_duration_ms: 0,
        }
    }

    pub fn get_node(&self, mac: &str) -> Option<&NetworkNode> {
        self.nodes.get(mac)
    }

    pub fn get_tsn_nodes(&self) -> Vec<&NetworkNode> {
        self.nodes.values().filter(|n| n.tsn_capable).collect()
    }

    pub fn get_ptp_nodes(&self) -> Vec<&NetworkNode> {
        self.nodes.values().filter(|n| n.ptp_role.is_some()).collect()
    }

    pub fn get_tsn_paths(&self) -> Vec<&NetworkLink> {
        self.links.values().filter(|l| l.is_tsn_path).collect()
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.links.clear();
        self.mac_to_ip.clear();
    }
}

impl Default for TopologyManager {
    fn default() -> Self {
        Self::new()
    }
}
