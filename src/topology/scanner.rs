//! Active network topology scanner
//! Uses ARP scanning, ICMP ping, and other techniques to discover devices

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU32, Ordering}};
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

use pnet::datalink::{self, Channel, NetworkInterface};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket};
use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, ArpPacket, MutableArpPacket};
use pnet::packet::Packet;
use pnet::util::MacAddr;

// NetworkNode and NodeType available from super if needed

/// Result of a network scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub hosts: Vec<DiscoveredHost>,
    pub scan_duration_ms: u64,
    pub total_ips_scanned: u32,
    pub hosts_found: u32,
    pub interface: String,
    pub network: String,
}

/// A discovered host from scanning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredHost {
    pub ip: String,
    pub mac: String,
    pub hostname: Option<String>,
    pub vendor: Option<String>,
    pub response_time_ms: f64,
    pub discovered_at: DateTime<Utc>,
}

/// Network topology scanner
pub struct TopologyScanner {
    interface_name: String,
    timeout_ms: u64,
    running: Arc<AtomicBool>,
    progress: Arc<AtomicU32>,
}

impl TopologyScanner {
    /// Create a new scanner for the specified interface
    pub fn new(interface: &str) -> Self {
        Self {
            interface_name: interface.to_string(),
            timeout_ms: 1000,
            running: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Set scan timeout in milliseconds
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Get progress (0-100)
    pub fn get_progress(&self) -> u32 {
        self.progress.load(Ordering::Relaxed)
    }

    /// Check if scan is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Stop the scan
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Perform ARP scan on local network
    pub fn arp_scan(&self, network: &str) -> Result<ScanResult, String> {
        self.running.store(true, Ordering::SeqCst);
        self.progress.store(0, Ordering::Relaxed);

        let start = Instant::now();

        // Get interface
        let interface = self.get_interface()?;
        let source_mac = interface.mac.ok_or("Interface has no MAC address")?;
        let source_ip = self.get_interface_ip(&interface)?;

        // Parse network CIDR
        let (network_addr, prefix_len) = self.parse_cidr(network)?;
        let host_count = 2u32.pow(32 - prefix_len as u32) - 2; // Exclude network and broadcast

        // Create channel
        let (mut tx, mut rx) = match datalink::channel(&interface, Default::default()) {
            Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => return Err("Unsupported channel type".to_string()),
            Err(e) => return Err(format!("Failed to create channel: {}", e)),
        };

        let mut hosts: HashMap<String, DiscoveredHost> = HashMap::new();
        let mut scanned = 0u32;

        // Generate target IPs
        let targets = self.generate_host_ips(network_addr, prefix_len);

        for (i, target_ip) in targets.iter().enumerate() {
            if !self.running.load(Ordering::SeqCst) {
                break;
            }

            // Update progress
            let progress = ((i as f64 / targets.len() as f64) * 100.0) as u32;
            self.progress.store(progress, Ordering::Relaxed);
            scanned += 1;

            // Send ARP request
            if let Err(_) = self.send_arp_request(&mut tx, source_mac, source_ip, *target_ip) {
                continue;
            }

            // Brief wait then check for responses
            std::thread::sleep(Duration::from_micros(100));

            // Check for responses (non-blocking)
            let deadline = Instant::now() + Duration::from_millis(10);
            while Instant::now() < deadline {
                if let Ok(packet) = rx.next() {
                    if let Some(host) = self.parse_arp_response(packet, source_mac) {
                        hosts.insert(host.ip.clone(), host);
                    }
                }
            }
        }

        // Final sweep for any remaining responses
        let final_deadline = Instant::now() + Duration::from_millis(self.timeout_ms);
        while Instant::now() < final_deadline && self.running.load(Ordering::SeqCst) {
            if let Ok(packet) = rx.next() {
                if let Some(host) = self.parse_arp_response(packet, source_mac) {
                    hosts.insert(host.ip.clone(), host);
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        self.progress.store(100, Ordering::Relaxed);
        self.running.store(false, Ordering::SeqCst);

        let duration = start.elapsed();
        let hosts_vec: Vec<DiscoveredHost> = hosts.into_values().collect();

        Ok(ScanResult {
            hosts_found: hosts_vec.len() as u32,
            hosts: hosts_vec,
            scan_duration_ms: duration.as_millis() as u64,
            total_ips_scanned: scanned,
            interface: self.interface_name.clone(),
            network: network.to_string(),
        })
    }

    /// Quick scan - only common IPs and gateway
    pub fn quick_scan(&self) -> Result<ScanResult, String> {
        self.running.store(true, Ordering::SeqCst);
        self.progress.store(0, Ordering::Relaxed);

        let start = Instant::now();

        let interface = self.get_interface()?;
        let source_mac = interface.mac.ok_or("Interface has no MAC address")?;
        let source_ip = self.get_interface_ip(&interface)?;

        let (mut tx, mut rx) = match datalink::channel(&interface, Default::default()) {
            Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => return Err("Unsupported channel type".to_string()),
            Err(e) => return Err(format!("Failed to create channel: {}", e)),
        };

        let mut hosts: HashMap<String, DiscoveredHost> = HashMap::new();

        // Common IPs to scan
        let network_base = Ipv4Addr::new(
            source_ip.octets()[0],
            source_ip.octets()[1],
            source_ip.octets()[2],
            0,
        );

        let common_suffixes = [1, 2, 100, 200, 254]; // Gateway and common DHCP ranges
        let mut targets: Vec<Ipv4Addr> = common_suffixes
            .iter()
            .map(|&suffix| {
                Ipv4Addr::new(
                    network_base.octets()[0],
                    network_base.octets()[1],
                    network_base.octets()[2],
                    suffix,
                )
            })
            .collect();

        // Also scan a range around the host IP
        let host_suffix = source_ip.octets()[3];
        for i in host_suffix.saturating_sub(5)..=host_suffix.saturating_add(5) {
            if i > 0 && i < 255 {
                let ip = Ipv4Addr::new(
                    network_base.octets()[0],
                    network_base.octets()[1],
                    network_base.octets()[2],
                    i,
                );
                if !targets.contains(&ip) {
                    targets.push(ip);
                }
            }
        }

        for (i, target_ip) in targets.iter().enumerate() {
            if !self.running.load(Ordering::SeqCst) {
                break;
            }

            let progress = ((i as f64 / targets.len() as f64) * 100.0) as u32;
            self.progress.store(progress, Ordering::Relaxed);

            let _ = self.send_arp_request(&mut tx, source_mac, source_ip, *target_ip);
            std::thread::sleep(Duration::from_millis(5));

            // Check responses
            if let Ok(packet) = rx.next() {
                if let Some(host) = self.parse_arp_response(packet, source_mac) {
                    hosts.insert(host.ip.clone(), host);
                }
            }
        }

        // Wait for remaining responses
        let deadline = Instant::now() + Duration::from_millis(500);
        while Instant::now() < deadline {
            if let Ok(packet) = rx.next() {
                if let Some(host) = self.parse_arp_response(packet, source_mac) {
                    hosts.insert(host.ip.clone(), host);
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        self.progress.store(100, Ordering::Relaxed);
        self.running.store(false, Ordering::SeqCst);

        let hosts_vec: Vec<DiscoveredHost> = hosts.into_values().collect();

        Ok(ScanResult {
            hosts_found: hosts_vec.len() as u32,
            hosts: hosts_vec,
            scan_duration_ms: start.elapsed().as_millis() as u64,
            total_ips_scanned: targets.len() as u32,
            interface: self.interface_name.clone(),
            network: format!("{}/24", network_base),
        })
    }

    fn get_interface(&self) -> Result<NetworkInterface, String> {
        datalink::interfaces()
            .into_iter()
            .find(|iface| iface.name == self.interface_name)
            .ok_or_else(|| format!("Interface {} not found", self.interface_name))
    }

    fn get_interface_ip(&self, interface: &NetworkInterface) -> Result<Ipv4Addr, String> {
        for ip in &interface.ips {
            if let IpAddr::V4(ipv4) = ip.ip() {
                if !ipv4.is_loopback() {
                    return Ok(ipv4);
                }
            }
        }
        Err("No IPv4 address on interface".to_string())
    }

    fn parse_cidr(&self, network: &str) -> Result<(Ipv4Addr, u8), String> {
        let parts: Vec<&str> = network.split('/').collect();
        if parts.len() != 2 {
            return Err("Invalid CIDR format".to_string());
        }

        let addr: Ipv4Addr = parts[0].parse()
            .map_err(|_| "Invalid IP address")?;
        let prefix: u8 = parts[1].parse()
            .map_err(|_| "Invalid prefix length")?;

        if prefix > 32 {
            return Err("Invalid prefix length".to_string());
        }

        Ok((addr, prefix))
    }

    fn generate_host_ips(&self, network: Ipv4Addr, prefix_len: u8) -> Vec<Ipv4Addr> {
        let mut ips = Vec::new();
        let network_int = u32::from(network);
        let host_bits = 32 - prefix_len;
        let host_count = 2u32.pow(host_bits as u32);

        // Skip network address (first) and broadcast (last)
        for i in 1..(host_count - 1) {
            let ip_int = (network_int & !(host_count - 1)) | i;
            ips.push(Ipv4Addr::from(ip_int));

            // Limit to reasonable scan size
            if ips.len() >= 1024 {
                break;
            }
        }

        ips
    }

    fn send_arp_request(
        &self,
        tx: &mut Box<dyn datalink::DataLinkSender>,
        source_mac: MacAddr,
        source_ip: Ipv4Addr,
        target_ip: Ipv4Addr,
    ) -> Result<(), String> {
        let mut ethernet_buffer = [0u8; 42]; // Ethernet (14) + ARP (28)
        let mut ethernet_packet = MutableEthernetPacket::new(&mut ethernet_buffer)
            .ok_or("Failed to create ethernet packet")?;

        ethernet_packet.set_destination(MacAddr::broadcast());
        ethernet_packet.set_source(source_mac);
        ethernet_packet.set_ethertype(EtherTypes::Arp);

        let mut arp_buffer = [0u8; 28];
        let mut arp_packet = MutableArpPacket::new(&mut arp_buffer)
            .ok_or("Failed to create ARP packet")?;

        arp_packet.set_hardware_type(ArpHardwareTypes::Ethernet);
        arp_packet.set_protocol_type(EtherTypes::Ipv4);
        arp_packet.set_hw_addr_len(6);
        arp_packet.set_proto_addr_len(4);
        arp_packet.set_operation(ArpOperations::Request);
        arp_packet.set_sender_hw_addr(source_mac);
        arp_packet.set_sender_proto_addr(source_ip);
        arp_packet.set_target_hw_addr(MacAddr::zero());
        arp_packet.set_target_proto_addr(target_ip);

        ethernet_packet.set_payload(arp_packet.packet());

        tx.send_to(ethernet_packet.packet(), None)
            .ok_or("Failed to send packet")?
            .map_err(|e| format!("Send error: {}", e))?;

        Ok(())
    }

    fn parse_arp_response(&self, packet: &[u8], our_mac: MacAddr) -> Option<DiscoveredHost> {
        let ethernet = EthernetPacket::new(packet)?;

        if ethernet.get_ethertype() != EtherTypes::Arp {
            return None;
        }

        let arp = ArpPacket::new(ethernet.payload())?;

        // Only process ARP replies
        if arp.get_operation() != ArpOperations::Reply {
            return None;
        }

        // Ignore our own packets
        if arp.get_sender_hw_addr() == our_mac {
            return None;
        }

        let sender_ip = arp.get_sender_proto_addr();
        let sender_mac = arp.get_sender_hw_addr();

        Some(DiscoveredHost {
            ip: sender_ip.to_string(),
            mac: format!("{}", sender_mac),
            hostname: None, // Could do reverse DNS here
            vendor: None,   // Could do OUI lookup here
            response_time_ms: 0.0, // Would need timing data
            discovered_at: Utc::now(),
        })
    }
}

/// Get local interface information
pub fn get_interfaces() -> Vec<InterfaceInfo> {
    datalink::interfaces()
        .into_iter()
        .filter(|iface| !iface.is_loopback() && iface.is_up())
        .map(|iface| {
            let ipv4: Vec<String> = iface.ips.iter()
                .filter_map(|ip| match ip.ip() {
                    IpAddr::V4(v4) => Some(v4.to_string()),
                    _ => None,
                })
                .collect();

            let ipv6: Vec<String> = iface.ips.iter()
                .filter_map(|ip| match ip.ip() {
                    IpAddr::V6(v6) => Some(v6.to_string()),
                    _ => None,
                })
                .collect();

            InterfaceInfo {
                name: iface.name.clone(),
                mac: iface.mac.map(|m| m.to_string()),
                ipv4_addresses: ipv4,
                ipv6_addresses: ipv6,
                is_up: iface.is_up(),
                is_running: iface.is_running(),
            }
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceInfo {
    pub name: String,
    pub mac: Option<String>,
    pub ipv4_addresses: Vec<String>,
    pub ipv6_addresses: Vec<String>,
    pub is_up: bool,
    pub is_running: bool,
}
