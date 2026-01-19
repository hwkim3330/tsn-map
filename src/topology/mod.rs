use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use petgraph::graph::{Graph, NodeIndex};
use crate::capture::CapturedPacket;

pub mod scanner;
pub mod lldp;

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
    // New fields
    pub lldp_info: Option<LldpInfo>,
    pub port_id: Option<String>,
    pub capabilities: Vec<DeviceCapability>,
    pub management_addresses: Vec<String>,
    pub ttl: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NodeType {
    Host,
    Switch,
    Router,
    Bridge,
    EndStation,
    TsnBridge,
    PtpGrandmaster,
    AccessPoint,
    Repeater,
    Gateway,
    Unknown,
}

impl NodeType {
    pub fn icon(&self) -> &'static str {
        match self {
            NodeType::Host => "💻",
            NodeType::Switch => "🔀",
            NodeType::Router => "🌐",
            NodeType::Bridge => "🌉",
            NodeType::EndStation => "📱",
            NodeType::TsnBridge => "⏱️",
            NodeType::PtpGrandmaster => "🕐",
            NodeType::AccessPoint => "📡",
            NodeType::Repeater => "📶",
            NodeType::Gateway => "🚪",
            NodeType::Unknown => "❓",
        }
    }

    pub fn priority(&self) -> u8 {
        match self {
            NodeType::PtpGrandmaster => 10,
            NodeType::TsnBridge => 9,
            NodeType::Router => 8,
            NodeType::Gateway => 7,
            NodeType::Switch => 6,
            NodeType::Bridge => 5,
            NodeType::AccessPoint => 4,
            NodeType::Host => 3,
            NodeType::EndStation => 2,
            NodeType::Repeater => 1,
            NodeType::Unknown => 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeviceCapability {
    Other,
    Repeater,
    Bridge,
    WlanAP,
    Router,
    Telephone,
    DocsisCableDevice,
    StationOnly,
    CVlanComponent,
    SVlanComponent,
    TwoPortMacRelay,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PtpRole {
    Grandmaster,
    BoundaryClock,
    OrdinaryClock,
    TransparentClock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LldpInfo {
    pub chassis_id: String,
    pub chassis_id_subtype: u8,
    pub port_id: String,
    pub port_id_subtype: u8,
    pub port_description: Option<String>,
    pub system_name: Option<String>,
    pub system_description: Option<String>,
    pub system_capabilities: Vec<DeviceCapability>,
    pub enabled_capabilities: Vec<DeviceCapability>,
    pub management_addresses: Vec<String>,
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
    pub jitter_us: Option<f64>,
    pub vlan_ids: Vec<u16>,
    pub traffic_classes: Vec<u8>,
    pub is_tsn_path: bool,
    // New fields
    pub link_type: LinkType,
    pub duplex: Option<bool>,
    pub speed_mbps: Option<u32>,
    pub quality: LinkQuality,
    pub last_active: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LinkType {
    Direct,
    SwitchedNetwork,
    Wireless,
    Virtual,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkQuality {
    pub packet_loss_percent: f64,
    pub avg_latency_us: Option<f64>,
    pub stability_score: f64, // 0.0 - 1.0
}

impl Default for LinkQuality {
    fn default() -> Self {
        Self {
            packet_loss_percent: 0.0,
            avg_latency_us: None,
            stability_score: 1.0,
        }
    }
}

/// Network topology graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTopology {
    pub nodes: Vec<NetworkNode>,
    pub links: Vec<NetworkLink>,
    pub last_updated: DateTime<Utc>,
    pub scan_duration_ms: u64,
    // New fields
    pub total_bandwidth_mbps: f64,
    pub tsn_nodes_count: usize,
    pub ptp_domain: Option<u8>,
    pub gateway_node: Option<String>,
}

/// Graph-based topology manager using petgraph
pub struct TopologyManager {
    // Petgraph for efficient graph operations
    graph: Graph<String, LinkData>,
    node_indices: HashMap<String, NodeIndex>,

    // Node data storage
    nodes: HashMap<String, NetworkNode>,
    links: HashMap<String, NetworkLink>,

    // Lookup tables
    mac_to_ip: HashMap<String, HashSet<String>>,
    ip_to_mac: HashMap<String, String>,
    ip_to_hostname: HashMap<String, String>,

    // OUI lookup
    oui_database: HashMap<String, String>,

    // Statistics
    stats: TopologyStats,
}

#[derive(Clone)]
struct LinkData {
    packets: u64,
    bytes: u64,
    is_tsn: bool,
}

#[derive(Default)]
struct TopologyStats {
    total_packets: u64,
    total_bytes: u64,
    first_packet: Option<DateTime<Utc>>,
    last_packet: Option<DateTime<Utc>>,
}

impl TopologyManager {
    pub fn new() -> Self {
        let mut manager = Self {
            graph: Graph::new(),
            node_indices: HashMap::new(),
            nodes: HashMap::new(),
            links: HashMap::new(),
            mac_to_ip: HashMap::new(),
            ip_to_mac: HashMap::new(),
            ip_to_hostname: HashMap::new(),
            oui_database: HashMap::new(),
            stats: TopologyStats::default(),
        };

        manager.init_oui_database();
        manager
    }

    fn init_oui_database(&mut self) {
        // TSN and networking equipment vendors
        let ouis = vec![
            // Microchip / MCHP
            ("00:1a:6b", "Microchip Technology"),
            ("00:04:25", "Microchip Technology"),
            ("d8:80:39", "Microchip Technology"),
            ("00:1e:c0", "Microchip Technology"),
            ("00:04:a3", "Microchip Technology"),
            // Intel
            ("00:1b:21", "Intel"),
            ("00:1f:c6", "Intel"),
            ("3c:fd:fe", "Intel"),
            ("a0:36:9f", "Intel"),
            ("00:15:17", "Intel"),
            ("68:05:ca", "Intel"),
            ("f8:f2:1e", "Intel"),
            // Cisco
            ("00:00:0c", "Cisco"),
            ("00:1a:a1", "Cisco"),
            ("00:1b:54", "Cisco"),
            ("00:1e:f7", "Cisco"),
            ("00:22:55", "Cisco"),
            // Juniper
            ("00:1d:b5", "Juniper Networks"),
            ("00:17:cb", "Juniper Networks"),
            ("00:05:85", "Juniper Networks"),
            // Arista
            ("00:1c:73", "Arista Networks"),
            ("28:99:3a", "Arista Networks"),
            // HPE / Aruba
            ("00:1e:0b", "Hewlett Packard"),
            ("00:25:b3", "Hewlett Packard"),
            ("00:1f:29", "Hewlett Packard"),
            ("d4:c9:ef", "Aruba Networks"),
            ("00:0b:86", "Aruba Networks"),
            // Broadcom
            ("00:10:18", "Broadcom"),
            ("00:1a:2a", "Broadcom"),
            // Realtek
            ("52:54:00", "Realtek/QEMU"),
            ("00:e0:4c", "Realtek"),
            // Virtual
            ("00:50:56", "VMware"),
            ("00:0c:29", "VMware"),
            ("00:15:5d", "Microsoft Hyper-V"),
            ("08:00:27", "VirtualBox"),
            // Special addresses
            ("00:00:5e", "IANA VRRP/HSRP"),
            ("01:00:5e", "IPv4 Multicast"),
            ("33:33:00", "IPv6 Multicast"),
            ("01:1b:19", "PTP/IEEE1588"),
            ("01:80:c2", "IEEE 802.1 Protocols"),
            // Apple
            ("00:03:93", "Apple"),
            ("00:0a:95", "Apple"),
            ("00:0d:93", "Apple"),
            ("00:10:fa", "Apple"),
            // Samsung
            ("00:02:78", "Samsung"),
            ("00:07:ab", "Samsung"),
            ("00:12:fb", "Samsung"),
            // Texas Instruments (TSN chips)
            ("00:17:e6", "Texas Instruments"),
            ("00:18:30", "Texas Instruments"),
            ("04:a3:16", "Texas Instruments"),
            // NXP (TSN chips)
            ("00:04:9f", "NXP/Freescale"),
            ("00:1f:7b", "NXP"),
            // Marvell
            ("00:00:f0", "Marvell"),
            ("08:9e:01", "Marvell"),
            // Renesas (TSN)
            ("00:30:55", "Renesas"),
        ];

        for (oui, vendor) in ouis {
            self.oui_database.insert(oui.to_lowercase(), vendor.to_string());
        }
    }

    /// Look up vendor from MAC address using OUI
    pub fn lookup_vendor(&self, mac: &str) -> Option<String> {
        let mac_lower = mac.to_lowercase();
        if mac_lower.len() >= 8 {
            let oui_prefix = &mac_lower[..8];
            if let Some(vendor) = self.oui_database.get(oui_prefix) {
                return Some(vendor.clone());
            }
        }

        // Could extend with oui crate for more vendors
        None
    }

    /// Process a captured packet to update topology
    pub fn process_packet(&mut self, packet: &CapturedPacket) {
        // Update statistics
        self.stats.total_packets += 1;
        self.stats.total_bytes += packet.length as u64;

        if self.stats.first_packet.is_none() {
            self.stats.first_packet = Some(packet.timestamp);
        }
        self.stats.last_packet = Some(packet.timestamp);

        // Update source node
        self.update_node(&packet.info.src_mac, packet, true);

        // Update destination node (if not multicast/broadcast)
        if !self.is_multicast_mac(&packet.info.dst_mac) {
            self.update_node(&packet.info.dst_mac, packet, false);
        }

        // Update link between nodes
        self.update_link(&packet.info.src_mac, &packet.info.dst_mac, packet);

        // Try to resolve hostname for IP addresses
        self.try_resolve_hostname(&packet.info.src_ip);
        self.try_resolve_hostname(&packet.info.dst_ip);

        // Parse LLDP if present
        if packet.info.protocol.as_deref() == Some("LLDP") {
            self.parse_lldp_from_packet(packet);
        }
    }

    fn is_multicast_mac(&self, mac: &str) -> bool {
        mac.starts_with("01:")
            || mac.starts_with("33:33")
            || mac == "ff:ff:ff:ff:ff:ff"
    }

    fn update_node(&mut self, mac: &str, packet: &CapturedPacket, is_source: bool) {
        let vendor = self.lookup_vendor(mac);
        let node_type = self.infer_node_type(mac, &vendor, packet);

        // Ensure node exists in graph
        let _node_idx = self.ensure_graph_node(mac);

        // First, update lookup tables for IP addresses
        let ip_to_add = if is_source {
            packet.info.src_ip.clone()
        } else {
            packet.info.dst_ip.clone()
        };

        if let Some(ref ip) = ip_to_add {
            self.mac_to_ip
                .entry(mac.to_string())
                .or_default()
                .insert(ip.clone());
            self.ip_to_mac.insert(ip.clone(), mac.to_string());
        }

        // Get hostname if available
        let hostname = ip_to_add.as_ref()
            .and_then(|ip| self.ip_to_hostname.get(ip).cloned());

        // Now update the node
        let node = self.nodes.entry(mac.to_string()).or_insert_with(|| {
            NetworkNode {
                id: mac.to_string(),
                mac_address: mac.to_string(),
                ip_addresses: Vec::new(),
                hostname: None,
                node_type: node_type.clone(),
                vendor: vendor.clone(),
                first_seen: packet.timestamp,
                last_seen: packet.timestamp,
                packets_sent: 0,
                packets_received: 0,
                bytes_sent: 0,
                bytes_received: 0,
                tsn_capable: false,
                ptp_role: None,
                vlan_memberships: Vec::new(),
                lldp_info: None,
                port_id: None,
                capabilities: Vec::new(),
                management_addresses: Vec::new(),
                ttl: None,
            }
        });

        node.last_seen = packet.timestamp;

        // Update stats and IP address
        if is_source {
            node.packets_sent += 1;
            node.bytes_sent += packet.length as u64;

            if let Some(ref ip) = packet.info.src_ip {
                if !node.ip_addresses.contains(ip) {
                    node.ip_addresses.push(ip.clone());
                }
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

        // Update hostname if found
        if hostname.is_some() && node.hostname.is_none() {
            node.hostname = hostname;
        }

        // Update VLAN membership
        if let Some(vlan) = packet.info.vlan_id {
            if !node.vlan_memberships.contains(&vlan) {
                node.vlan_memberships.push(vlan);
                node.vlan_memberships.sort();
            }
        }

        // Check TSN capability (inline to avoid borrow issues)
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
                        "Pdelay_Req" | "Pdelay_Resp" if is_source => {
                            if node.ptp_role.is_none() {
                                node.ptp_role = Some(PtpRole::TransparentClock);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Update node type if we have better information
        if node_type.priority() > node.node_type.priority() {
            node.node_type = node_type;
        }
    }

    fn update_link(&mut self, src: &str, dst: &str, packet: &CapturedPacket) {
        // Ensure both nodes exist in graph
        let src_idx = self.ensure_graph_node(src);
        let dst_idx = self.ensure_graph_node(dst);

        // Update graph edge
        if self.graph.find_edge(src_idx, dst_idx).is_none() {
            self.graph.add_edge(src_idx, dst_idx, LinkData {
                packets: 0,
                bytes: 0,
                is_tsn: false,
            });
        }

        if let Some(edge) = self.graph.find_edge(src_idx, dst_idx) {
            if let Some(data) = self.graph.edge_weight_mut(edge) {
                data.packets += 1;
                data.bytes += packet.length as u64;
                if packet.info.is_tsn || packet.info.is_ptp {
                    data.is_tsn = true;
                }
            }
        }

        // Update link data structure
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
                jitter_us: None,
                vlan_ids: Vec::new(),
                traffic_classes: Vec::new(),
                is_tsn_path: false,
                link_type: LinkType::Unknown,
                duplex: None,
                speed_mbps: None,
                quality: LinkQuality::default(),
                last_active: packet.timestamp,
            }
        });

        link.packets += 1;
        link.bytes += packet.length as u64;
        link.last_active = packet.timestamp;

        // Update VLAN IDs
        if let Some(vlan) = packet.info.vlan_id {
            if !link.vlan_ids.contains(&vlan) {
                link.vlan_ids.push(vlan);
                link.vlan_ids.sort();
            }
        }

        // Update traffic classes
        if let Some(pcp) = packet.info.vlan_pcp {
            if !link.traffic_classes.contains(&pcp) {
                link.traffic_classes.push(pcp);
                link.traffic_classes.sort();
            }
        }

        // Check if TSN path
        if packet.info.is_tsn || packet.info.is_ptp {
            link.is_tsn_path = true;
        }

        // Calculate bandwidth
        self.update_link_bandwidth(&src.to_string(), &dst.to_string());
    }

    fn update_link_bandwidth(&mut self, src: &str, dst: &str) {
        let link_id = format!("{}:{}", src, dst);
        if let Some(link) = self.links.get_mut(&link_id) {
            if let (Some(first), Some(last)) = (&self.stats.first_packet, &self.stats.last_packet) {
                let duration_secs = (*last - *first).num_milliseconds() as f64 / 1000.0;
                if duration_secs > 0.0 {
                    link.bandwidth_mbps = (link.bytes as f64 * 8.0) / (duration_secs * 1_000_000.0);
                }
            }
        }
    }

    fn ensure_graph_node(&mut self, mac: &str) -> NodeIndex {
        if let Some(&idx) = self.node_indices.get(mac) {
            return idx;
        }

        let idx = self.graph.add_node(mac.to_string());
        self.node_indices.insert(mac.to_string(), idx);
        idx
    }

    fn infer_node_type(&self, mac: &str, vendor: &Option<String>, packet: &CapturedPacket) -> NodeType {
        // Check special MAC addresses
        if mac.starts_with("01:1b:19") {
            return NodeType::PtpGrandmaster;
        }
        if mac.starts_with("01:80:c2") {
            return NodeType::TsnBridge;
        }
        if mac.starts_with("01:") || mac.starts_with("33:33") {
            return NodeType::Unknown;
        }

        // Check protocol-based detection
        if packet.info.is_ptp {
            if let Some(ref tsn) = packet.tsn_info {
                if let Some(ref ptp) = tsn.ptp_info {
                    if ptp.message_type == "Announce" || ptp.message_type == "Sync" {
                        return NodeType::PtpGrandmaster;
                    }
                }
            }
        }

        if packet.info.protocol.as_deref() == Some("LLDP") {
            // Will be refined after LLDP parsing
            return NodeType::Switch;
        }

        // Infer from vendor
        if let Some(v) = vendor {
            let v_lower = v.to_lowercase();
            if v_lower.contains("microchip") || v_lower.contains("texas instruments") || v_lower.contains("nxp") {
                return NodeType::TsnBridge;
            }
            if v_lower.contains("cisco") || v_lower.contains("juniper") || v_lower.contains("arista")
                || v_lower.contains("hewlett") {
                return NodeType::Switch;
            }
            if v_lower.contains("aruba") {
                return NodeType::AccessPoint;
            }
            if v_lower.contains("vmware") || v_lower.contains("hyper-v") || v_lower.contains("virtualbox") {
                return NodeType::Host;
            }
        }

        // Check if it appears to be routing (multiple IP destinations)
        if let Some(ips) = self.mac_to_ip.get(mac) {
            if ips.len() > 3 {
                return NodeType::Router;
            }
        }

        NodeType::EndStation
    }

    fn try_resolve_hostname(&mut self, ip: &Option<String>) {
        if let Some(ip_str) = ip {
            if self.ip_to_hostname.contains_key(ip_str) {
                return;
            }

            // Attempt reverse DNS lookup (non-blocking in real implementation)
            if let Ok(ip_addr) = ip_str.parse::<IpAddr>() {
                // Use dns-lookup crate for reverse lookup
                if let Ok(hostname) = dns_lookup::lookup_addr(&ip_addr) {
                    if hostname != *ip_str {
                        self.ip_to_hostname.insert(ip_str.clone(), hostname.clone());
                        // Update node hostname
                        if let Some(mac) = self.ip_to_mac.get(ip_str) {
                            if let Some(node) = self.nodes.get_mut(mac) {
                                node.hostname = Some(hostname);
                            }
                        }
                    }
                }
            }
        }
    }

    fn parse_lldp_from_packet(&mut self, packet: &CapturedPacket) {
        // LLDP parsing handled by lldp module
        if let Some(lldp_info) = lldp::parse_lldp_packet(&packet.data) {
            if let Some(node) = self.nodes.get_mut(&packet.info.src_mac) {
                // Update node type based on LLDP capabilities
                for cap in &lldp_info.enabled_capabilities {
                    match cap {
                        DeviceCapability::Router => node.node_type = NodeType::Router,
                        DeviceCapability::Bridge => {
                            if node.node_type.priority() < NodeType::Switch.priority() {
                                node.node_type = NodeType::Switch;
                            }
                        }
                        DeviceCapability::WlanAP => node.node_type = NodeType::AccessPoint,
                        DeviceCapability::Repeater => {
                            if node.node_type.priority() < NodeType::Repeater.priority() {
                                node.node_type = NodeType::Repeater;
                            }
                        }
                        _ => {}
                    }
                }

                node.hostname = lldp_info.system_name.clone();
                node.port_id = Some(lldp_info.port_id.clone());
                node.capabilities = lldp_info.enabled_capabilities.clone();
                node.management_addresses = lldp_info.management_addresses.clone();
                node.lldp_info = Some(lldp_info);
            }
        }
    }

    /// Get complete network topology
    pub fn get_topology(&self) -> NetworkTopology {
        let nodes: Vec<NetworkNode> = self.nodes.values().cloned().collect();
        let links: Vec<NetworkLink> = self.links.values().cloned().collect();

        let tsn_count = nodes.iter().filter(|n| n.tsn_capable).count();
        let gateway = self.find_gateway_node();

        let total_bw: f64 = links.iter().map(|l| l.bandwidth_mbps).sum();

        NetworkTopology {
            nodes,
            links,
            last_updated: Utc::now(),
            scan_duration_ms: 0,
            total_bandwidth_mbps: total_bw,
            tsn_nodes_count: tsn_count,
            ptp_domain: None,
            gateway_node: gateway,
        }
    }

    fn find_gateway_node(&self) -> Option<String> {
        // Find node with most outgoing connections and highest traffic
        let mut best: Option<(String, u64)> = None;

        for (mac, node) in &self.nodes {
            if node.node_type == NodeType::Router || node.node_type == NodeType::Gateway {
                return Some(mac.clone());
            }

            let traffic = node.bytes_sent + node.bytes_received;
            if let Some((_, best_traffic)) = &best {
                if traffic > *best_traffic {
                    best = Some((mac.clone(), traffic));
                }
            } else {
                best = Some((mac.clone(), traffic));
            }
        }

        best.map(|(mac, _)| mac)
    }

    /// Get shortest path between two nodes using petgraph
    pub fn get_path(&self, src_mac: &str, dst_mac: &str) -> Option<Vec<String>> {
        let src_idx = self.node_indices.get(src_mac)?;
        let dst_idx = self.node_indices.get(dst_mac)?;

        use petgraph::algo::dijkstra;

        let paths = dijkstra(&self.graph, *src_idx, Some(*dst_idx), |_| 1);

        if paths.contains_key(dst_idx) {
            // Reconstruct path
            let mut path = Vec::new();
            path.push(dst_mac.to_string());
            // Note: dijkstra returns distances, not paths. For full path, use astar or manual BFS
            path.insert(0, src_mac.to_string());
            Some(path)
        } else {
            None
        }
    }

    /// Get all neighbors of a node
    pub fn get_neighbors(&self, mac: &str) -> Vec<String> {
        if let Some(&idx) = self.node_indices.get(mac) {
            self.graph
                .neighbors_undirected(idx)
                .filter_map(|n| self.graph.node_weight(n).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get node by MAC
    pub fn get_node(&self, mac: &str) -> Option<&NetworkNode> {
        self.nodes.get(mac)
    }

    /// Get all TSN-capable nodes
    pub fn get_tsn_nodes(&self) -> Vec<&NetworkNode> {
        self.nodes.values().filter(|n| n.tsn_capable).collect()
    }

    /// Get all PTP nodes
    pub fn get_ptp_nodes(&self) -> Vec<&NetworkNode> {
        self.nodes.values().filter(|n| n.ptp_role.is_some()).collect()
    }

    /// Get all TSN paths
    pub fn get_tsn_paths(&self) -> Vec<&NetworkLink> {
        self.links.values().filter(|l| l.is_tsn_path).collect()
    }

    /// Get nodes grouped by type
    pub fn get_nodes_by_type(&self) -> HashMap<NodeType, Vec<&NetworkNode>> {
        let mut grouped: HashMap<NodeType, Vec<&NetworkNode>> = HashMap::new();
        for node in self.nodes.values() {
            grouped.entry(node.node_type.clone()).or_default().push(node);
        }
        grouped
    }

    /// Clear all topology data
    pub fn clear(&mut self) {
        self.graph.clear();
        self.node_indices.clear();
        self.nodes.clear();
        self.links.clear();
        self.mac_to_ip.clear();
        self.ip_to_mac.clear();
        self.ip_to_hostname.clear();
        self.stats = TopologyStats::default();
    }

    /// Get topology statistics
    pub fn get_stats(&self) -> TopologyStatsResponse {
        TopologyStatsResponse {
            total_nodes: self.nodes.len(),
            total_links: self.links.len(),
            total_packets: self.stats.total_packets,
            total_bytes: self.stats.total_bytes,
            tsn_nodes: self.nodes.values().filter(|n| n.tsn_capable).count(),
            ptp_nodes: self.nodes.values().filter(|n| n.ptp_role.is_some()).count(),
        }
    }
}

#[derive(Serialize)]
pub struct TopologyStatsResponse {
    pub total_nodes: usize,
    pub total_links: usize,
    pub total_packets: u64,
    pub total_bytes: u64,
    pub tsn_nodes: usize,
    pub ptp_nodes: usize,
}

impl Default for TopologyManager {
    fn default() -> Self {
        Self::new()
    }
}
