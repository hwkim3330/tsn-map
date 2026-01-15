use std::fs::File;
use std::io::{Write, Read, BufWriter};
use std::path::Path;
use chrono::{DateTime, Utc, TimeZone};
use super::{CapturedPacket, PacketInfo};

const PCAP_MAGIC: u32 = 0xa1b2c3d4;
const PCAP_VERSION_MAJOR: u16 = 2;
const PCAP_VERSION_MINOR: u16 = 4;
const PCAP_THISZONE: i32 = 0;
const PCAP_SIGFIGS: u32 = 0;
const PCAP_SNAPLEN: u32 = 65535;
const PCAP_LINKTYPE_ETHERNET: u32 = 1;

pub struct PcapHandler;

impl PcapHandler {
    /// Save packets to a pcap file
    pub fn save_pcap(
        packets: &[CapturedPacket],
        path: &Path,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // Write pcap global header
        writer.write_all(&PCAP_MAGIC.to_le_bytes())?;
        writer.write_all(&PCAP_VERSION_MAJOR.to_le_bytes())?;
        writer.write_all(&PCAP_VERSION_MINOR.to_le_bytes())?;
        writer.write_all(&PCAP_THISZONE.to_le_bytes())?;
        writer.write_all(&PCAP_SIGFIGS.to_le_bytes())?;
        writer.write_all(&PCAP_SNAPLEN.to_le_bytes())?;
        writer.write_all(&PCAP_LINKTYPE_ETHERNET.to_le_bytes())?;

        let mut count = 0;
        for packet in packets {
            // Write packet header
            let ts_sec = packet.timestamp.timestamp() as u32;
            let ts_usec = packet.timestamp.timestamp_subsec_micros();
            let incl_len = packet.data.len() as u32;
            let orig_len = packet.length;

            writer.write_all(&ts_sec.to_le_bytes())?;
            writer.write_all(&ts_usec.to_le_bytes())?;
            writer.write_all(&incl_len.to_le_bytes())?;
            writer.write_all(&orig_len.to_le_bytes())?;

            // Write packet data
            writer.write_all(&packet.data)?;
            count += 1;
        }

        writer.flush()?;
        Ok(count)
    }

    /// Load packets from a pcap file
    pub fn load_pcap(
        path: &Path,
    ) -> Result<Vec<CapturedPacket>, Box<dyn std::error::Error + Send + Sync>> {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        if buffer.len() < 24 {
            return Err("Invalid pcap file: too short".into());
        }

        // Read and validate magic number
        let magic = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
        let (is_swapped, is_nano) = match magic {
            0xa1b2c3d4 => (false, false), // Standard pcap, microseconds
            0xd4c3b2a1 => (true, false),  // Swapped pcap, microseconds
            0xa1b23c4d => (false, true),  // Standard pcap, nanoseconds
            0x4d3cb2a1 => (true, true),   // Swapped pcap, nanoseconds
            _ => return Err(format!("Invalid pcap magic number: 0x{:08X}", magic).into()),
        };

        // Skip global header
        let mut offset = 24;
        let mut packets = Vec::new();
        let mut packet_id = 0u64;

        while offset + 16 <= buffer.len() {
            // Read packet header
            let ts_sec = if is_swapped {
                u32::from_be_bytes([buffer[offset], buffer[offset + 1], buffer[offset + 2], buffer[offset + 3]])
            } else {
                u32::from_le_bytes([buffer[offset], buffer[offset + 1], buffer[offset + 2], buffer[offset + 3]])
            };

            let ts_sub = if is_swapped {
                u32::from_be_bytes([buffer[offset + 4], buffer[offset + 5], buffer[offset + 6], buffer[offset + 7]])
            } else {
                u32::from_le_bytes([buffer[offset + 4], buffer[offset + 5], buffer[offset + 6], buffer[offset + 7]])
            };

            let incl_len = if is_swapped {
                u32::from_be_bytes([buffer[offset + 8], buffer[offset + 9], buffer[offset + 10], buffer[offset + 11]])
            } else {
                u32::from_le_bytes([buffer[offset + 8], buffer[offset + 9], buffer[offset + 10], buffer[offset + 11]])
            };

            let orig_len = if is_swapped {
                u32::from_be_bytes([buffer[offset + 12], buffer[offset + 13], buffer[offset + 14], buffer[offset + 15]])
            } else {
                u32::from_le_bytes([buffer[offset + 12], buffer[offset + 13], buffer[offset + 14], buffer[offset + 15]])
            };

            offset += 16;

            if offset + incl_len as usize > buffer.len() {
                break;
            }

            // Read packet data
            let data = buffer[offset..offset + incl_len as usize].to_vec();
            offset += incl_len as usize;

            // Create timestamp
            let nanos = if is_nano {
                ts_sub
            } else {
                ts_sub * 1000
            };
            let timestamp = Utc.timestamp_opt(ts_sec as i64, nanos).unwrap();

            // Create packet
            let mut packet = CapturedPacket::from_raw(packet_id, &data, timestamp);
            packet.length = orig_len;
            packets.push(packet);
            packet_id += 1;
        }

        Ok(packets)
    }

    /// Save packets to bytes (in-memory pcap)
    pub fn save_pcap_to_bytes(
        packets: &[CapturedPacket],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let mut buffer = Vec::new();

        // Write pcap global header
        buffer.extend_from_slice(&PCAP_MAGIC.to_le_bytes());
        buffer.extend_from_slice(&PCAP_VERSION_MAJOR.to_le_bytes());
        buffer.extend_from_slice(&PCAP_VERSION_MINOR.to_le_bytes());
        buffer.extend_from_slice(&PCAP_THISZONE.to_le_bytes());
        buffer.extend_from_slice(&PCAP_SIGFIGS.to_le_bytes());
        buffer.extend_from_slice(&PCAP_SNAPLEN.to_le_bytes());
        buffer.extend_from_slice(&PCAP_LINKTYPE_ETHERNET.to_le_bytes());

        for packet in packets {
            // Write packet header
            let ts_sec = packet.timestamp.timestamp() as u32;
            let ts_usec = packet.timestamp.timestamp_subsec_micros();
            let incl_len = packet.data.len() as u32;
            let orig_len = packet.length;

            buffer.extend_from_slice(&ts_sec.to_le_bytes());
            buffer.extend_from_slice(&ts_usec.to_le_bytes());
            buffer.extend_from_slice(&incl_len.to_le_bytes());
            buffer.extend_from_slice(&orig_len.to_le_bytes());

            // Write packet data
            buffer.extend_from_slice(&packet.data);
        }

        Ok(buffer)
    }

    /// Load packets from bytes (in-memory pcap)
    pub fn load_pcap_from_bytes(
        data: &[u8],
    ) -> Result<Vec<CapturedPacket>, Box<dyn std::error::Error + Send + Sync>> {
        if data.len() < 24 {
            return Err("Invalid pcap file: too short".into());
        }

        // Read and validate magic number
        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let (is_swapped, is_nano) = match magic {
            0xa1b2c3d4 => (false, false),
            0xd4c3b2a1 => (true, false),
            0xa1b23c4d => (false, true),
            0x4d3cb2a1 => (true, true),
            _ => return Err(format!("Invalid pcap magic number: 0x{:08X}", magic).into()),
        };

        let mut offset = 24;
        let mut packets = Vec::new();
        let mut packet_id = 0u64;

        while offset + 16 <= data.len() {
            let ts_sec = if is_swapped {
                u32::from_be_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]])
            } else {
                u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]])
            };

            let ts_sub = if is_swapped {
                u32::from_be_bytes([data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]])
            } else {
                u32::from_le_bytes([data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]])
            };

            let incl_len = if is_swapped {
                u32::from_be_bytes([data[offset + 8], data[offset + 9], data[offset + 10], data[offset + 11]])
            } else {
                u32::from_le_bytes([data[offset + 8], data[offset + 9], data[offset + 10], data[offset + 11]])
            };

            let orig_len = if is_swapped {
                u32::from_be_bytes([data[offset + 12], data[offset + 13], data[offset + 14], data[offset + 15]])
            } else {
                u32::from_le_bytes([data[offset + 12], data[offset + 13], data[offset + 14], data[offset + 15]])
            };

            offset += 16;

            if offset + incl_len as usize > data.len() {
                break;
            }

            let pkt_data = data[offset..offset + incl_len as usize].to_vec();
            offset += incl_len as usize;

            let nanos = if is_nano { ts_sub } else { ts_sub * 1000 };
            let timestamp = Utc.timestamp_opt(ts_sec as i64, nanos).unwrap();

            let mut packet = CapturedPacket::from_raw(packet_id, &pkt_data, timestamp);
            packet.length = orig_len;
            packets.push(packet);
            packet_id += 1;
        }

        Ok(packets)
    }

    /// Export packets to CSV format
    pub fn export_csv(
        packets: &[CapturedPacket],
        path: &Path,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let mut file = File::create(path)?;

        // Write header
        writeln!(
            file,
            "id,timestamp,length,src_mac,dst_mac,ethertype,vlan_id,vlan_pcp,src_ip,dst_ip,protocol,src_port,dst_port,is_tsn,is_ptp,tsn_type"
        )?;

        for packet in packets {
            let tsn_type = packet.tsn_info.as_ref()
                .map(|t| format!("{:?}", t.tsn_type))
                .unwrap_or_else(|| "None".to_string());

            writeln!(
                file,
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                packet.id,
                packet.timestamp.to_rfc3339(),
                packet.length,
                packet.info.src_mac,
                packet.info.dst_mac,
                packet.info.ethertype_name,
                packet.info.vlan_id.map(|v| v.to_string()).unwrap_or_default(),
                packet.info.vlan_pcp.map(|v| v.to_string()).unwrap_or_default(),
                packet.info.src_ip.as_deref().unwrap_or(""),
                packet.info.dst_ip.as_deref().unwrap_or(""),
                packet.info.protocol.as_deref().unwrap_or(""),
                packet.info.src_port.map(|v| v.to_string()).unwrap_or_default(),
                packet.info.dst_port.map(|v| v.to_string()).unwrap_or_default(),
                packet.info.is_tsn,
                packet.info.is_ptp,
                tsn_type,
            )?;
        }

        Ok(packets.len())
    }
}
