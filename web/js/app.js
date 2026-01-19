// NetMap - Network Visualization Application
// Focus: nmap + Wireshark features with better visualization

const API_BASE = '';

// Application State
const state = {
    packets: [],
    filteredPackets: [],
    selectedPacket: null,
    isCapturing: false,
    topology: { nodes: [], links: [] },
    hosts: new Map(), // IP -> host info
    protocols: new Map(), // protocol -> count
    conversations: new Map(), // src-dst pair -> stats
    eventSource: null,
    charts: {},
    stats: {
        packets_captured: 0,
        bytes_captured: 0,
        start_time: null,
    },
    filter: '',
    autoScroll: true,
    layoutMode: 'force',
    selectedHost: null,
    // Pagination
    currentPage: 1,
    pageSize: 100,
    totalPages: 1,
};

// Initialize Application
document.addEventListener('DOMContentLoaded', () => {
    initializeUI();
    initializeCharts();
    initializeTestCharts();
    initializeColumnResize();
    loadStatus();
    setupEventListeners();
    startPolling();
    refreshTopology();
});

// UI Initialization
function initializeUI() {
    // Tab switching
    document.querySelectorAll('.tab').forEach(tab => {
        tab.addEventListener('click', () => {
            document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
            document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
            tab.classList.add('active');
            document.getElementById(`tab-${tab.dataset.tab}`).classList.add('active');

            if (tab.dataset.tab === 'topology') {
                setTimeout(renderTopology, 100);
            } else if (tab.dataset.tab === 'stats') {
                updateAllCharts();
            } else if (tab.dataset.tab === 'hosts') {
                renderHostsList();
            } else if (tab.dataset.tab === 'detail') {
                // Show placeholder if no packet selected
                if (!state.selectedPacket) {
                    document.getElementById('detail-placeholder').style.display = 'flex';
                    document.getElementById('detail-content').style.display = 'none';
                }
            } else if (tab.dataset.tab === 'latency') {
                // Resize chart when tab becomes visible
                if (state.charts.latency) {
                    setTimeout(() => state.charts.latency.resize(), 100);
                }
            } else if (tab.dataset.tab === 'throughput') {
                // Resize chart when tab becomes visible
                if (state.charts.throughput) {
                    setTimeout(() => state.charts.throughput.resize(), 100);
                }
            }
        });
    });
}

// Event Listeners
function setupEventListeners() {
    // Capture controls
    document.getElementById('btn-start').addEventListener('click', startCapture);
    document.getElementById('btn-stop').addEventListener('click', stopCapture);
    document.getElementById('btn-clear').addEventListener('click', clearAll);

    // File operations
    document.getElementById('btn-save').addEventListener('click', savePcap);
    document.getElementById('btn-load').addEventListener('click', loadPcap);
    document.getElementById('btn-export-csv').addEventListener('click', exportCSV);
    document.getElementById('pcap-file-input').addEventListener('change', handlePcapFileSelect);

    // Interface selection
    document.getElementById('interface-name').addEventListener('click', showInterfaceModal);
    document.getElementById('btn-interface-cancel').addEventListener('click', hideInterfaceModal);

    // Filter
    document.getElementById('packet-filter').addEventListener('keyup', (e) => {
        if (e.key === 'Enter') applyFilter();
    });
    document.getElementById('btn-apply-filter').addEventListener('click', applyFilter);
    document.getElementById('btn-clear-filter').addEventListener('click', clearFilter);

    // Pagination
    document.getElementById('btn-first-page').addEventListener('click', () => goToPage(1));
    document.getElementById('btn-prev-page').addEventListener('click', () => goToPage(state.currentPage - 1));
    document.getElementById('btn-next-page').addEventListener('click', () => goToPage(state.currentPage + 1));
    document.getElementById('btn-last-page').addEventListener('click', () => goToPage(state.totalPages));
    document.getElementById('page-size').addEventListener('change', (e) => {
        state.pageSize = parseInt(e.target.value);
        state.currentPage = 1;
        renderPacketList();
    });

    // Topology controls
    document.getElementById('btn-zoom-in').addEventListener('click', () => zoomTopology(1.2));
    document.getElementById('btn-zoom-out').addEventListener('click', () => zoomTopology(0.8));
    document.getElementById('btn-zoom-fit').addEventListener('click', fitTopology);
    document.getElementById('show-labels').addEventListener('change', renderTopology);
    document.getElementById('auto-refresh').addEventListener('change', (e) => {
        state.autoRefresh = e.target.checked;
    });

    // Host search and sort
    document.getElementById('host-search').addEventListener('input', renderHostsList);
    document.getElementById('host-sort').addEventListener('change', renderHostsList);

    // Tester controls
    document.getElementById('btn-ping-start')?.addEventListener('click', startPingTest);
    document.getElementById('btn-throughput-start')?.addEventListener('click', startThroughputTest);
    document.getElementById('btn-cbs-apply')?.addEventListener('click', applyCbsConfig);
    document.getElementById('btn-tas-apply')?.addEventListener('click', applyTasConfig);
}

// API Functions
async function apiCall(endpoint, method = 'GET', body = null) {
    const options = {
        method,
        headers: { 'Content-Type': 'application/json' },
    };
    if (body) options.body = JSON.stringify(body);

    try {
        const response = await fetch(`${API_BASE}${endpoint}`, options);
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        return await response.json();
    } catch (error) {
        console.error('API Error:', error);
        updateConnectionStatus(false);
        return null;
    }
}

async function loadStatus() {
    const data = await apiCall('/api/status');
    if (data) {
        document.getElementById('interface-name').textContent = data.interface;
        updateCaptureStatus(data.is_capturing);
        updateConnectionStatus(true);
        if (data.is_capturing) {
            startPacketStream();
        }
    }
}

async function loadPackets() {
    const data = await apiCall('/api/packets?limit=1000');
    if (data && data.packets) {
        data.packets.forEach(packet => processPacket(packet, false));
        renderPacketList();
        updateAllCharts();
    }
}

// Capture Control
async function startCapture() {
    const result = await apiCall('/api/capture/start', 'POST');
    if (result && result.success) {
        updateCaptureStatus(true);
        state.stats.start_time = Date.now();
        startPacketStream();
    }
}

async function stopCapture() {
    const result = await apiCall('/api/capture/stop', 'POST');
    if (result && result.success) {
        updateCaptureStatus(false);
        stopPacketStream();
    }
}

function updateCaptureStatus(capturing) {
    state.isCapturing = capturing;
    const statusEl = document.getElementById('capture-status');
    const startBtn = document.getElementById('btn-start');
    const stopBtn = document.getElementById('btn-stop');

    if (capturing) {
        statusEl.textContent = 'Capturing';
        statusEl.className = 'status status-capturing';
        startBtn.disabled = true;
        stopBtn.disabled = false;
    } else {
        statusEl.textContent = 'Stopped';
        statusEl.className = 'status status-stopped';
        startBtn.disabled = false;
        stopBtn.disabled = true;
    }
}

function updateConnectionStatus(connected) {
    const dot = document.getElementById('connection-status');
    const text = document.getElementById('connection-text');
    if (connected) {
        dot.className = 'status-dot connected';
        text.textContent = 'Connected';
    } else {
        dot.className = 'status-dot';
        text.textContent = 'Disconnected';
    }
}

// Packet Stream (SSE)
function startPacketStream() {
    if (state.eventSource) {
        state.eventSource.close();
    }

    state.eventSource = new EventSource('/api/packets/stream');

    state.eventSource.onmessage = (event) => {
        try {
            const packet = JSON.parse(event.data);
            processPacket(packet, true);
        } catch (e) {
            console.error('Parse error:', e);
        }
    };

    state.eventSource.onerror = () => {
        console.error('SSE connection error');
        updateConnectionStatus(false);
    };

    state.eventSource.onopen = () => {
        updateConnectionStatus(true);
    };
}

function stopPacketStream() {
    if (state.eventSource) {
        state.eventSource.close();
        state.eventSource = null;
    }
}

// Packet Processing - Batch processing for better performance
let pendingPackets = [];
let updateScheduled = false;

function processPacket(packet, live = true) {
    // Add to packet list
    state.packets.push(packet);
    state.stats.packets_captured++;
    state.stats.bytes_captured += packet.length;

    // Update protocol statistics
    const proto = packet.info.protocol || packet.info.ethertype_name || 'Unknown';
    state.protocols.set(proto, (state.protocols.get(proto) || 0) + 1);

    // Update host information
    updateHostInfo(packet);

    // Update conversation statistics
    updateConversation(packet);

    // Limit packet list size
    if (state.packets.length > 50000) {
        state.packets = state.packets.slice(-40000);
    }

    if (live) {
        // Batch packet updates for performance
        if (matchesFilter(packet, state.filter)) {
            pendingPackets.push(packet);
        }

        // Schedule batch update
        if (!updateScheduled) {
            updateScheduled = true;
            requestAnimationFrame(flushPendingPackets);
        }
    }
}

function flushPendingPackets() {
    updateScheduled = false;
    if (pendingPackets.length === 0) return;

    // Process all pending packets
    const packetsToAdd = pendingPackets;
    pendingPackets = [];

    packetsToAdd.forEach(packet => {
        state.filteredPackets.push(packet);
    });

    // Update pagination state
    const newTotalPages = Math.max(1, Math.ceil(state.filteredPackets.length / state.pageSize));

    // If on last page, append new packet rows
    if (state.currentPage === state.totalPages) {
        const tbody = document.getElementById('packet-tbody');
        const fragment = document.createDocumentFragment();

        packetsToAdd.forEach(packet => {
            const currentPagePackets = state.filteredPackets.length - (state.currentPage - 1) * state.pageSize;
            if (currentPagePackets <= state.pageSize) {
                const row = createPacketRow(packet);
                fragment.appendChild(row);
            }
        });

        tbody.appendChild(fragment);

        // Auto-scroll
        if (state.autoScroll) {
            const packetList = document.querySelector('.packet-list');
            packetList.scrollTop = packetList.scrollHeight;
        }
    }

    state.totalPages = newTotalPages;
    updatePaginationUI();
    updateCounters();
}

function createPacketRow(packet) {
    const row = document.createElement('tr');
    row.className = getProtocolClass(packet);
    row.dataset.id = packet.id;
    row.onclick = () => selectPacket(packet);

    const time = formatTime(packet.timestamp);
    const info = getPacketInfo(packet);
    const srcAddr = shortenAddress(packet.info.src_ip || packet.info.src_mac);
    const dstAddr = shortenAddress(packet.info.dst_ip || packet.info.dst_mac);

    row.innerHTML = `
        <td>${packet.id}</td>
        <td>${time}</td>
        <td title="${packet.info.src_ip || packet.info.src_mac || ''}">${srcAddr}</td>
        <td title="${packet.info.dst_ip || packet.info.dst_mac || ''}">${dstAddr}</td>
        <td><span class="proto-badge proto-${(packet.info.protocol || packet.info.ethertype_name || '').toLowerCase()}">${packet.info.protocol || packet.info.ethertype_name || '-'}</span></td>
        <td>${packet.length}</td>
        <td class="info-cell">${info}</td>
    `;

    return row;
}

// Shorten IPv6 and long addresses for display
function shortenAddress(addr) {
    if (!addr) return '-';
    return addr;  // Let CSS handle truncation with text-overflow
}

function updateHostInfo(packet) {
    const srcIP = packet.info.src_ip;
    const dstIP = packet.info.dst_ip;
    const srcMAC = packet.info.src_mac;
    const dstMAC = packet.info.dst_mac;
    const now = Date.now();

    if (srcIP && srcIP !== '0.0.0.0') {
        const host = state.hosts.get(srcIP) || {
            ip: srcIP,
            mac: srcMAC,
            packets_sent: 0,
            packets_recv: 0,
            bytes_sent: 0,
            bytes_recv: 0,
            protocols: new Set(),
            ports: new Set(),
            first_seen: now,
            last_seen: now,
        };
        host.packets_sent++;
        host.bytes_sent += packet.length;
        host.last_seen = now;
        host.mac = srcMAC || host.mac;
        if (packet.info.protocol) host.protocols.add(packet.info.protocol);
        if (packet.info.src_port) host.ports.add(packet.info.src_port);
        state.hosts.set(srcIP, host);
    }

    if (dstIP && dstIP !== '0.0.0.0' && dstIP !== '255.255.255.255') {
        const host = state.hosts.get(dstIP) || {
            ip: dstIP,
            mac: dstMAC,
            packets_sent: 0,
            packets_recv: 0,
            bytes_sent: 0,
            bytes_recv: 0,
            protocols: new Set(),
            ports: new Set(),
            first_seen: now,
            last_seen: now,
        };
        host.packets_recv++;
        host.bytes_recv += packet.length;
        host.last_seen = now;
        host.mac = dstMAC || host.mac;
        if (packet.info.protocol) host.protocols.add(packet.info.protocol);
        if (packet.info.dst_port) host.ports.add(packet.info.dst_port);
        state.hosts.set(dstIP, host);
    }
}

function updateConversation(packet) {
    const srcIP = packet.info.src_ip;
    const dstIP = packet.info.dst_ip;
    if (!srcIP || !dstIP) return;

    // Normalize conversation key (smaller IP first)
    const key = srcIP < dstIP ? `${srcIP}-${dstIP}` : `${dstIP}-${srcIP}`;

    const conv = state.conversations.get(key) || {
        ipA: srcIP < dstIP ? srcIP : dstIP,
        ipB: srcIP < dstIP ? dstIP : srcIP,
        packets: 0,
        bytes: 0,
        protocols: new Set(),
    };
    conv.packets++;
    conv.bytes += packet.length;
    if (packet.info.protocol) conv.protocols.add(packet.info.protocol);
    state.conversations.set(key, conv);
}

function updateCounters() {
    document.getElementById('packet-count').textContent = `${state.stats.packets_captured.toLocaleString()} packets`;
    document.getElementById('byte-count').textContent = formatBytes(state.stats.bytes_captured);
}

// Packet List Rendering - Optimized with document fragments
function renderPacketList() {
    const tbody = document.getElementById('packet-tbody');
    tbody.innerHTML = '';

    state.filteredPackets = state.filter
        ? state.packets.filter(p => matchesFilter(p, state.filter))
        : [...state.packets];

    // Calculate pagination
    const totalPackets = state.filteredPackets.length;
    state.totalPages = Math.max(1, Math.ceil(totalPackets / state.pageSize));

    // Adjust current page if out of bounds
    if (state.currentPage > state.totalPages) {
        state.currentPage = state.totalPages;
    }

    // Get packets for current page
    const startIdx = (state.currentPage - 1) * state.pageSize;
    const endIdx = startIdx + state.pageSize;
    const displayPackets = state.filteredPackets.slice(startIdx, endIdx);

    // Use document fragment for batch DOM update
    const fragment = document.createDocumentFragment();
    displayPackets.forEach(packet => {
        const row = createPacketRow(packet);
        fragment.appendChild(row);
    });
    tbody.appendChild(fragment);

    // Update pagination UI
    updatePaginationUI();
}

function goToPage(page) {
    if (page < 1 || page > state.totalPages) return;
    state.currentPage = page;
    renderPacketList();

    // Scroll to top of packet list
    document.querySelector('.packet-list').scrollTop = 0;
}

function updatePaginationUI() {
    document.getElementById('current-page').textContent = state.currentPage;
    document.getElementById('total-pages').textContent = state.totalPages;
    document.getElementById('filtered-count').textContent = state.filteredPackets.length.toLocaleString();

    // Enable/disable buttons
    document.getElementById('btn-first-page').disabled = state.currentPage <= 1;
    document.getElementById('btn-prev-page').disabled = state.currentPage <= 1;
    document.getElementById('btn-next-page').disabled = state.currentPage >= state.totalPages;
    document.getElementById('btn-last-page').disabled = state.currentPage >= state.totalPages;
}

// appendPacketRow - now uses createPacketRow for consistency
function appendPacketRow(packet, live = false) {
    const tbody = document.getElementById('packet-tbody');
    const row = createPacketRow(packet);
    tbody.appendChild(row);

    // Auto-scroll only for live packets on last page
    if (live && state.autoScroll && state.currentPage === state.totalPages) {
        const packetList = document.querySelector('.packet-list');
        packetList.scrollTop = packetList.scrollHeight;
    }
}

function getProtocolClass(packet) {
    const proto = (packet.info.protocol || packet.info.ethertype_name || '').toLowerCase();
    if (proto === 'tcp') return 'proto-tcp';
    if (proto === 'udp') return 'proto-udp';
    if (proto === 'arp' || proto === 'rarp') return 'proto-arp';
    if (proto === 'icmp' || proto === 'icmpv6') return 'proto-icmp';
    if (proto === 'dns') return 'proto-dns';
    if (proto === 'http' || proto === 'https') return 'proto-http';
    if (proto === 'tls' || proto === 'ssl') return 'proto-tls';
    if (proto === 'igmp' || proto === 'pim') return 'proto-multicast';
    if (proto === 'vrrp' || proto === 'ospf' || proto === 'eigrp') return 'proto-routing';
    if (proto === 'ptp' || proto === 'lldp') return 'proto-network';
    if (proto === 'rrcp' || proto === 'loopback') return 'proto-l2';
    if (proto === 'esp' || proto === 'ah' || proto === 'macsec') return 'proto-security';
    if (proto === 'hopopt' || proto === 'hop-by-hop') return 'proto-network';  // IPv6 Hop-by-Hop Options
    if (proto.startsWith('0x')) return 'proto-l2';  // Unknown EtherType
    return '';
}

function getPacketInfo(packet) {
    const info = packet.info;

    // TCP with ports/flags
    if (info.protocol === 'TCP' && info.src_port && info.dst_port) {
        const flags = [];
        if (info.tcp_flags) {
            if (info.tcp_flags.syn) flags.push('SYN');
            if (info.tcp_flags.ack) flags.push('ACK');
            if (info.tcp_flags.fin) flags.push('FIN');
            if (info.tcp_flags.rst) flags.push('RST');
            if (info.tcp_flags.psh) flags.push('PSH');
        }
        const flagStr = flags.length ? ` [${flags.join(',')}]` : '';
        return `${info.src_port} → ${info.dst_port}${flagStr}`;
    }

    // UDP with ports
    if (info.protocol === 'UDP' && info.src_port && info.dst_port) {
        // Check for known protocols
        if (info.dst_port === 53 || info.src_port === 53) return `DNS ${info.src_port} → ${info.dst_port}`;
        if (info.dst_port === 67 || info.dst_port === 68) return `DHCP ${info.src_port} → ${info.dst_port}`;
        if (info.dst_port === 123 || info.src_port === 123) return `NTP ${info.src_port} → ${info.dst_port}`;
        if (info.dst_port === 319 || info.dst_port === 320) return `PTP ${info.src_port} → ${info.dst_port}`;
        if (info.dst_port === 514 || info.src_port === 514) return `Syslog ${info.src_port} → ${info.dst_port}`;
        if (info.dst_port === 161 || info.src_port === 161) return `SNMP ${info.src_port} → ${info.dst_port}`;
        return `${info.src_port} → ${info.dst_port}`;
    }

    // ARP
    if (info.ethertype_name === 'ARP') {
        return info.arp_op === 1 ? `Who has ${info.dst_ip}? Tell ${info.src_ip}` : `${info.src_ip} is at ${info.src_mac}`;
    }

    // ICMP
    if (info.protocol === 'ICMP' || info.protocol === 'ICMPv6') {
        const types = { 0: 'Echo Reply', 8: 'Echo Request', 3: 'Dest Unreachable', 11: 'TTL Exceeded', 5: 'Redirect' };
        return types[info.icmp_type] || `Type ${info.icmp_type || 'N/A'}`;
    }

    // IGMP
    if (info.protocol === 'IGMP') {
        return `Membership ${info.dst_ip || 'Report'}`;
    }

    // VRRP
    if (info.protocol === 'VRRP') {
        return `Virtual Router ${info.dst_ip || 'Advertisement'}`;
    }

    // Routing protocols
    if (info.protocol === 'OSPF') {
        return `OSPF Router ${info.src_ip}`;
    }

    // PTP (Layer 2)
    if (info.ethertype_name === 'PTP' || info.is_ptp) {
        return packet.tsn_info?.ptp_info?.message_type || 'PTP Message';
    }

    // LLDP
    if (info.ethertype_name === 'LLDP') {
        return `LLDP from ${info.src_mac}`;
    }

    // L2 protocols
    if (info.ethertype_name === 'RRCP') {
        return `Realtek Remote Control`;
    }

    if (info.ethertype_name === 'Loopback') {
        return `Loop Detection Frame`;
    }

    // VLAN info
    if (info.vlan_id) {
        const vlanInfo = `VLAN ${info.vlan_id} PCP ${info.vlan_pcp}`;
        if (info.protocol) {
            return `${vlanInfo} → ${info.protocol}`;
        }
        return vlanInfo;
    }

    return info.ethertype_name || info.protocol || '-';
}

function formatTime(timestamp) {
    const date = new Date(timestamp);
    return date.toLocaleTimeString('ko-KR', {
        hour12: false,
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
        fractionalSecondDigits: 3
    });
}

// Filter
function applyFilter() {
    state.filter = document.getElementById('packet-filter').value.trim();
    renderPacketList();
}

function clearFilter() {
    document.getElementById('packet-filter').value = '';
    state.filter = '';
    renderPacketList();
}

function matchesFilter(packet, filter) {
    if (!filter) return true;

    const f = filter.toLowerCase();
    const info = packet.info;
    const proto = (info.protocol || '').toLowerCase();
    const ethertype = (info.ethertype_name || '').toLowerCase();

    // Protocol filter (L3/L4)
    if (f === 'tcp') return proto === 'tcp';
    if (f === 'udp') return proto === 'udp';
    if (f === 'icmp') return proto === 'icmp' || proto === 'icmpv6';
    if (f === 'igmp') return proto === 'igmp';
    if (f === 'vrrp') return proto === 'vrrp';
    if (f === 'ospf') return proto === 'ospf';
    if (f === 'gre') return proto === 'gre';
    if (f === 'esp') return proto === 'esp';
    if (f === 'sctp') return proto === 'sctp';

    // EtherType filter (L2)
    if (f === 'arp') return ethertype === 'arp';
    if (f === 'rarp') return ethertype === 'rarp';
    if (f === 'ipv4') return ethertype === 'ipv4';
    if (f === 'ipv6') return ethertype === 'ipv6';
    if (f === 'ptp') return ethertype === 'ptp' || info.is_ptp;
    if (f === 'lldp') return ethertype === 'lldp';
    if (f === 'rrcp') return ethertype === 'rrcp';
    if (f === 'loopback') return ethertype === 'loopback';
    if (f === 'macsec') return ethertype === 'macsec';
    if (f === 'vlan') return info.vlan_id !== null && info.vlan_id !== undefined;

    // Application layer
    if (f === 'dns') return info.src_port === 53 || info.dst_port === 53;
    if (f === 'http') return info.dst_port === 80 || info.src_port === 80;
    if (f === 'https' || f === 'tls') return info.dst_port === 443 || info.src_port === 443;

    // IP filter
    if (f.startsWith('ip.addr==')) {
        const ip = f.substring(9);
        return info.src_ip === ip || info.dst_ip === ip;
    }
    if (f.startsWith('ip.src==')) {
        return info.src_ip === f.substring(8);
    }
    if (f.startsWith('ip.dst==')) {
        return info.dst_ip === f.substring(8);
    }

    // Port filter
    if (f.startsWith('port==') || f.startsWith('tcp.port==') || f.startsWith('udp.port==')) {
        const port = parseInt(f.split('==')[1]);
        return info.src_port === port || info.dst_port === port;
    }

    // General text search
    const text = JSON.stringify(packet).toLowerCase();
    return text.includes(f);
}

// Packet Detail
function selectPacket(packet) {
    state.selectedPacket = packet;

    // Update selection highlighting
    document.querySelectorAll('#packet-tbody tr').forEach(row => {
        row.classList.toggle('selected', row.dataset.id == packet.id);
    });

    // Update footer info immediately
    document.getElementById('selected-packet-info').textContent =
        `#${packet.id} | ${packet.info.protocol || packet.info.ethertype_name} | ${packet.length} bytes`;

    // Auto-switch to Detail tab
    switchTab('detail');

    // Show detail content
    document.getElementById('detail-placeholder').style.display = 'none';
    document.getElementById('detail-content').style.display = 'block';

    // Fill frame info
    const frameInfo = document.getElementById('frame-info');
    frameInfo.innerHTML = `
        <div class="detail-row"><span>Packet #:</span><span>${packet.id}</span></div>
        <div class="detail-row"><span>Capture Time:</span><span>${new Date(packet.timestamp).toISOString()}</span></div>
        <div class="detail-row"><span>Length:</span><span>${packet.length} bytes</span></div>
    `;

    // Fill ethernet info
    const ethInfo = document.getElementById('eth-info');
    ethInfo.innerHTML = `
        <div class="detail-row"><span>Source MAC:</span><span>${packet.info.src_mac}</span></div>
        <div class="detail-row"><span>Destination MAC:</span><span>${packet.info.dst_mac}</span></div>
        <div class="detail-row"><span>EtherType:</span><span>${packet.info.ethertype_name} (0x${packet.info.ethertype?.toString(16).padStart(4, '0') || '0000'})</span></div>
        ${packet.info.vlan_id ? `<div class="detail-row"><span>VLAN ID:</span><span>${packet.info.vlan_id}</span></div>` : ''}
        ${packet.info.vlan_pcp !== undefined ? `<div class="detail-row"><span>VLAN PCP:</span><span>${packet.info.vlan_pcp}</span></div>` : ''}
    `;

    // Fill IP info
    const ipSection = document.getElementById('ip-section');
    const ipInfo = document.getElementById('ip-info');
    if (packet.info.src_ip) {
        ipSection.style.display = 'block';
        ipInfo.innerHTML = `
            <div class="detail-row"><span>Source IP:</span><span>${packet.info.src_ip}</span></div>
            <div class="detail-row"><span>Destination IP:</span><span>${packet.info.dst_ip}</span></div>
            ${packet.info.ttl ? `<div class="detail-row"><span>TTL:</span><span>${packet.info.ttl}</span></div>` : ''}
            ${packet.info.ip_protocol ? `<div class="detail-row"><span>Protocol:</span><span>${packet.info.ip_protocol}</span></div>` : ''}
        `;
    } else {
        ipSection.style.display = 'none';
    }

    // Fill transport layer info
    const transportSection = document.getElementById('transport-section');
    const transportInfo = document.getElementById('transport-info');
    if (packet.info.src_port) {
        transportSection.style.display = 'block';
        let html = `
            <div class="detail-row"><span>Protocol:</span><span>${packet.info.protocol}</span></div>
            <div class="detail-row"><span>Source Port:</span><span>${packet.info.src_port}</span></div>
            <div class="detail-row"><span>Destination Port:</span><span>${packet.info.dst_port}</span></div>
        `;
        if (packet.info.protocol === 'TCP') {
            if (packet.info.seq_num !== undefined) {
                html += `<div class="detail-row"><span>Sequence:</span><span>${packet.info.seq_num}</span></div>`;
            }
            if (packet.info.ack_num !== undefined) {
                html += `<div class="detail-row"><span>Acknowledgment:</span><span>${packet.info.ack_num}</span></div>`;
            }
            if (packet.info.tcp_flags) {
                const flags = packet.info.tcp_flags;
                const flagStr = ['SYN', 'ACK', 'FIN', 'RST', 'PSH', 'URG']
                    .filter(f => flags[f.toLowerCase()])
                    .join(', ');
                html += `<div class="detail-row"><span>Flags:</span><span>${flagStr || 'None'}</span></div>`;
            }
        }
        transportInfo.innerHTML = html;
    } else {
        transportSection.style.display = 'none';
    }

    // Hex dump
    renderHexDump(packet.data);
}

function renderHexDump(data) {
    const hexDump = document.getElementById('hex-dump');
    if (!data || data.length === 0) {
        hexDump.textContent = 'No data';
        return;
    }

    let output = '';
    const maxBytes = Math.min(data.length, 512);

    for (let i = 0; i < maxBytes; i += 16) {
        const offset = i.toString(16).padStart(8, '0');
        const bytes = data.slice(i, Math.min(i + 16, maxBytes));
        const hex = bytes.map(b => b.toString(16).padStart(2, '0')).join(' ');
        const ascii = bytes.map(b => b >= 32 && b < 127 ? String.fromCharCode(b) : '.').join('');
        output += `${offset}  ${hex.padEnd(47)}  |${ascii}|\n`;
    }

    if (data.length > maxBytes) {
        output += `\n... ${data.length - maxBytes} more bytes ...`;
    }

    hexDump.textContent = output;
}

// Charts
function initializeCharts() {
    const chartOptions = {
        responsive: true,
        maintainAspectRatio: false,
        animation: { duration: 0 },
        plugins: {
            legend: {
                labels: { color: '#e6edf3', font: { size: 11 } }
            }
        }
    };

    // Protocol Pie Chart
    const protocolCtx = document.getElementById('protocol-pie-chart').getContext('2d');
    state.charts.protocol = new Chart(protocolCtx, {
        type: 'doughnut',
        data: {
            labels: [],
            datasets: [{
                data: [],
                backgroundColor: [
                    '#2f81f7', '#58a6ff', '#79c0ff', '#a5d6ff', '#3fb950',
                    '#a371f7', '#d29922', '#f85149', '#8b949e', '#6e7681'
                ]
            }]
        },
        options: {
            ...chartOptions,
            plugins: {
                ...chartOptions.plugins,
                legend: {
                    position: 'right',
                    labels: {
                        color: '#f5f5f7',
                        font: { size: 11 },
                        boxWidth: 12,
                        padding: 8,
                        generateLabels: function(chart) {
                            const data = chart.data;
                            if (data.labels.length && data.datasets.length) {
                                return data.labels.map((label, i) => ({
                                    text: label.length > 12 ? label.substring(0, 12) + '...' : label,
                                    fillStyle: data.datasets[0].backgroundColor[i],
                                    fontColor: '#f5f5f7',
                                    hidden: false,
                                    index: i
                                }));
                            }
                            return [];
                        }
                    }
                }
            }
        }
    });

    // Traffic Line Chart
    const trafficCtx = document.getElementById('traffic-line-chart').getContext('2d');
    state.charts.traffic = new Chart(trafficCtx, {
        type: 'line',
        data: {
            labels: [],
            datasets: [
                {
                    label: 'pps',
                    data: [],
                    borderColor: '#2f81f7',
                    backgroundColor: 'rgba(47, 129, 247, 0.1)',
                    tension: 0.4,
                    fill: true,
                    yAxisID: 'y'
                },
                {
                    label: 'KB/s',
                    data: [],
                    borderColor: '#3fb950',
                    backgroundColor: 'rgba(63, 185, 80, 0.1)',
                    tension: 0.4,
                    fill: true,
                    yAxisID: 'y1'
                }
            ]
        },
        options: {
            ...chartOptions,
            scales: {
                x: { grid: { color: '#30363d' }, ticks: { color: '#8b949e', maxTicksLimit: 10 } },
                y: {
                    position: 'left',
                    grid: { color: '#30363d' },
                    ticks: { color: '#2f81f7' },
                    beginAtZero: true,
                    suggestedMax: 100
                },
                y1: {
                    position: 'right',
                    grid: { drawOnChartArea: false },
                    ticks: { color: '#3fb950' },
                    beginAtZero: true,
                    suggestedMax: 100
                }
            }
        }
    });

    // Conversations Bar Chart
    const convCtx = document.getElementById('conversations-bar-chart').getContext('2d');
    state.charts.conversations = new Chart(convCtx, {
        type: 'bar',
        data: {
            labels: [],
            datasets: [{
                label: 'Packets',
                data: [],
                backgroundColor: '#2f81f7'
            }]
        },
        options: {
            ...chartOptions,
            indexAxis: 'y',
            scales: {
                x: { grid: { color: '#30363d' }, ticks: { color: '#8b949e' }, beginAtZero: true },
                y: {
                    grid: { color: '#30363d' },
                    ticks: {
                        color: '#e6edf3',
                        font: { size: 9 },
                        callback: function(value, index) {
                            const label = this.getLabelForValue(value);
                            return label.length > 20 ? label.substring(0, 20) + '...' : label;
                        }
                    }
                }
            }
        }
    });

    // Packet Size Distribution Chart
    const sizeCtx = document.getElementById('packet-size-chart').getContext('2d');
    state.charts.packetSize = new Chart(sizeCtx, {
        type: 'bar',
        data: {
            labels: ['0-64', '65-128', '129-256', '257-512', '513-1024', '1025-1518', '>1518'],
            datasets: [{
                label: 'Packets',
                data: [0, 0, 0, 0, 0, 0, 0],
                backgroundColor: '#58a6ff'
            }]
        },
        options: {
            ...chartOptions,
            scales: {
                x: { grid: { color: '#30363d' }, ticks: { color: '#8b949e' } },
                y: { grid: { color: '#30363d' }, ticks: { color: '#8b949e' }, beginAtZero: true }
            }
        }
    });
}

function updateAllCharts() {
    updateProtocolChart();
    updateConversationsChart();
    updatePacketSizeChart();
}

function updateProtocolChart() {
    const sorted = [...state.protocols.entries()]
        .sort((a, b) => b[1] - a[1])
        .slice(0, 10);

    state.charts.protocol.data.labels = sorted.map(([proto]) => proto);
    state.charts.protocol.data.datasets[0].data = sorted.map(([, count]) => count);
    state.charts.protocol.update('none');
}

function updateConversationsChart() {
    const sorted = [...state.conversations.entries()]
        .sort((a, b) => b[1].packets - a[1].packets)
        .slice(0, 10);

    state.charts.conversations.data.labels = sorted.map(([, conv]) =>
        `${conv.ipA.substring(0, 15)} ↔ ${conv.ipB.substring(0, 15)}`
    );
    state.charts.conversations.data.datasets[0].data = sorted.map(([, conv]) => conv.packets);
    state.charts.conversations.update('none');
}

function updatePacketSizeChart() {
    const sizes = [0, 0, 0, 0, 0, 0, 0];
    state.packets.forEach(p => {
        const len = p.length;
        if (len <= 64) sizes[0]++;
        else if (len <= 128) sizes[1]++;
        else if (len <= 256) sizes[2]++;
        else if (len <= 512) sizes[3]++;
        else if (len <= 1024) sizes[4]++;
        else if (len <= 1518) sizes[5]++;
        else sizes[6]++;
    });
    state.charts.packetSize.data.datasets[0].data = sizes;
    state.charts.packetSize.update('none');
}

// Traffic history for real-time chart
const trafficHistory = [];
setInterval(() => {
    const now = Date.now();
    const oneSecAgo = now - 1000;

    // Count packets in last second
    const recentPackets = state.packets.filter(p => new Date(p.timestamp).getTime() > oneSecAgo);
    const pps = recentPackets.length;
    const bps = recentPackets.reduce((sum, p) => sum + p.length, 0) / 1024;

    trafficHistory.push({ time: now, pps, bps });

    // Keep last 60 seconds
    while (trafficHistory.length > 60) trafficHistory.shift();

    // Update chart
    if (state.charts.traffic) {
        state.charts.traffic.data.labels = trafficHistory.map((_, i) =>
            i === trafficHistory.length - 1 ? 'now' : `-${trafficHistory.length - 1 - i}s`
        );
        state.charts.traffic.data.datasets[0].data = trafficHistory.map(h => h.pps);
        state.charts.traffic.data.datasets[1].data = trafficHistory.map(h => h.bps.toFixed(1));
        state.charts.traffic.update('none');
    }
}, 1000);

// Hosts List
function renderHostsList() {
    const container = document.getElementById('hosts-list');
    const searchTerm = document.getElementById('host-search').value.toLowerCase();
    const sortBy = document.getElementById('host-sort').value;

    let hosts = [...state.hosts.values()];

    // Filter
    if (searchTerm) {
        hosts = hosts.filter(h =>
            h.ip.toLowerCase().includes(searchTerm) ||
            (h.mac && h.mac.toLowerCase().includes(searchTerm))
        );
    }

    // Sort
    hosts.sort((a, b) => {
        if (sortBy === 'packets') return (b.packets_sent + b.packets_recv) - (a.packets_sent + a.packets_recv);
        if (sortBy === 'bytes') return (b.bytes_sent + b.bytes_recv) - (a.bytes_sent + a.bytes_recv);
        if (sortBy === 'last_seen') return b.last_seen - a.last_seen;
        return 0;
    });

    container.innerHTML = hosts.slice(0, 100).map(host => {
        const isIPv6 = host.ip.includes(':');
        const isSelected = state.selectedHost === host.ip;
        return `
        <div class="host-card${isSelected ? ' selected' : ''}" onclick="selectHost('${host.ip}')">
            <div class="host-header">
                <span class="host-ip${isIPv6 ? ' ipv6' : ''}">${host.ip}</span>
                <span class="host-type">${getHostType(host)}</span>
            </div>
            <div class="host-mac">${host.mac || 'Unknown'}</div>
            <div class="host-stats">
                <div class="stat-item">
                    <span class="stat-label">TX</span>
                    <span class="stat-value">${host.packets_sent.toLocaleString()} pkts / ${formatBytes(host.bytes_sent)}</span>
                </div>
                <div class="stat-item">
                    <span class="stat-label">RX</span>
                    <span class="stat-value">${host.packets_recv.toLocaleString()} pkts / ${formatBytes(host.bytes_recv)}</span>
                </div>
                <div class="stat-item">
                    <span class="stat-label">Protocols</span>
                    <span class="stat-value">${[...host.protocols].slice(0, 5).join(', ') || '-'}</span>
                </div>
                <div class="stat-item">
                    <span class="stat-label">Ports</span>
                    <span class="stat-value">${[...host.ports].slice(0, 8).join(', ') || '-'}</span>
                </div>
            </div>
            <div class="host-time">
                Last seen: ${formatTimeAgo(host.last_seen)}
            </div>
        </div>
    `}).join('');
}

function selectHost(ip) {
    state.selectedHost = (state.selectedHost === ip) ? null : ip;
    renderHostsList();
    if (state.selectedHost) {
        filterByHost(ip);
    }
}

function getHostType(host) {
    const ip = host.ip;
    if (ip.startsWith('192.168.') || ip.startsWith('10.') || ip.startsWith('172.16.')) {
        if (ip.endsWith('.1') || ip.endsWith('.254')) return 'Gateway';
        return 'Local';
    }
    if (ip === '255.255.255.255' || ip.endsWith('.255')) return 'Broadcast';
    if (ip.startsWith('224.') || ip.startsWith('239.')) return 'Multicast';
    return 'Remote';
}

function filterByHost(ip) {
    document.getElementById('packet-filter').value = `ip.addr==${ip}`;
    applyFilter();
    // Switch to packet list panel (you can implement tab switching here if needed)
}

function formatTimeAgo(timestamp) {
    const diff = Date.now() - timestamp;
    if (diff < 1000) return 'just now';
    if (diff < 60000) return `${Math.floor(diff / 1000)}s ago`;
    if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
    return `${Math.floor(diff / 3600000)}h ago`;
}

// Topology
let svg, simulation, zoom, topologyG;
let nodePositions = {};  // Store node positions between updates
let lastTopologyHash = '';  // Track changes

async function refreshTopology() {
    const data = await apiCall('/api/topology');
    if (data && data.success) {
        // Check if topology actually changed
        const newHash = JSON.stringify(data.data.nodes.map(n => n.id).sort());
        if (newHash === lastTopologyHash && state.topology.nodes.length > 0) {
            // Just update stats, don't re-render
            updateTopologyStats();
            return;
        }
        lastTopologyHash = newHash;
        state.topology = data.data;
        renderTopology();
    }
}

function renderTopology() {
    updateTopologyStats();

    const container = document.getElementById('topology-graph');
    const width = container.clientWidth || 600;
    const height = container.clientHeight || 400;

    // Limit nodes to top 30 for cleaner display
    let allNodes = state.topology.nodes.map(n => ({ ...n }));
    allNodes.sort((a, b) => (b.packets_sent + b.packets_received) - (a.packets_sent + a.packets_received));
    const nodes = allNodes.slice(0, 30);
    const nodeIds = new Set(nodes.map(n => n.id));

    // Restore saved positions
    nodes.forEach(n => {
        if (nodePositions[n.id]) {
            n.x = nodePositions[n.id].x;
            n.y = nodePositions[n.id].y;
            n.fx = nodePositions[n.id].fx;
            n.fy = nodePositions[n.id].fy;
        }
    });

    const links = state.topology.links
        .filter(l => nodeIds.has(l.source) && nodeIds.has(l.target))
        .map(l => ({ ...l }));

    // Initialize SVG only once
    if (!svg || container.querySelector('svg') === null) {
        container.innerHTML = '';
        svg = d3.select('#topology-graph')
            .append('svg')
            .attr('width', width)
            .attr('height', height);

        zoom = d3.zoom()
            .scaleExtent([0.1, 4])
            .on('zoom', (event) => {
                topologyG.attr('transform', event.transform);
            });

        svg.call(zoom);
        topologyG = svg.append('g');
    } else {
        svg.attr('width', width).attr('height', height);
    }

    if (nodes.length === 0) {
        topologyG.selectAll('*').remove();
        svg.selectAll('.empty-msg').remove();
        svg.append('text')
            .attr('class', 'empty-msg')
            .attr('x', width / 2)
            .attr('y', height / 2)
            .attr('text-anchor', 'middle')
            .attr('fill', '#8b949e')
            .text('Start capture to see network topology');
        return;
    }

    svg.selectAll('.empty-msg').remove();

    // Stop existing simulation
    if (simulation) {
        simulation.stop();
    }

    // Create simulation with lower alpha for smoother updates
    const isUpdate = Object.keys(nodePositions).length > 0;
    simulation = d3.forceSimulation(nodes)
        .force('link', d3.forceLink(links).id(d => d.id).distance(100))
        .force('charge', d3.forceManyBody().strength(-300))
        .force('center', d3.forceCenter(width / 2, height / 2))
        .force('collision', d3.forceCollide().radius(35))
        .alpha(isUpdate ? 0.1 : 0.8)  // Low alpha for updates
        .alphaDecay(0.05);

    // Update links with enter/update/exit
    const link = topologyG.selectAll('line.link')
        .data(links, d => d.id);

    link.exit().remove();

    const linkEnter = link.enter()
        .append('line')
        .attr('class', 'link')
        .attr('stroke', '#58a6ff')
        .attr('stroke-opacity', 0.5);

    link.merge(linkEnter)
        .attr('stroke-width', d => Math.min(Math.log(d.packets + 1) * 0.5 + 1, 3));

    // Update nodes with enter/update/exit
    const node = topologyG.selectAll('g.node')
        .data(nodes, d => d.id);

    node.exit().remove();

    const nodeEnter = node.enter()
        .append('g')
        .attr('class', 'node')
        .style('cursor', 'pointer')
        .call(d3.drag()
            .on('start', dragstarted)
            .on('drag', dragged)
            .on('end', dragended));

    nodeEnter.append('circle')
        .attr('stroke', '#30363d')
        .attr('stroke-width', 2);

    nodeEnter.append('text')
        .attr('class', 'node-label')
        .attr('dy', 4)
        .attr('fill', '#c9d1d9')
        .attr('font-size', '10px');

    nodeEnter.append('title');

    // Update all nodes (enter + update)
    const allNodes2 = node.merge(nodeEnter);

    allNodes2.select('circle')
        .attr('r', d => getNodeRadius(d))
        .attr('fill', d => getNodeColor(d));

    const showLabels = document.getElementById('show-labels').checked;
    allNodes2.select('text.node-label')
        .attr('dx', d => getNodeRadius(d) + 4)
        .text(d => showLabels ? (d.ip_addresses?.[0] || d.mac_address?.substring(0, 8) || '') : '');

    allNodes2.select('title')
        .text(d => {
            const ips = d.ip_addresses?.join(', ') || 'Unknown';
            return `IP: ${ips}\nMAC: ${d.mac_address || 'Unknown'}\nType: ${d.node_type || 'Unknown'}\nVendor: ${d.vendor || 'Unknown'}`;
        });

    allNodes2.on('click', function(event, d) {
        const ip = d.ip_addresses?.[0];
        if (ip) {
            filterByHost(ip);
            showNotification(`Filtering by ${ip}`, 'info');
        }
    });

    const allLinks = topologyG.selectAll('line.link');

    simulation.on('tick', () => {
        allLinks
            .attr('x1', d => d.source.x)
            .attr('y1', d => d.source.y)
            .attr('x2', d => d.target.x)
            .attr('y2', d => d.target.y);

        allNodes2.attr('transform', d => `translate(${d.x},${d.y})`);
    });

    // Save positions when simulation ends
    simulation.on('end', () => {
        nodes.forEach(n => {
            nodePositions[n.id] = { x: n.x, y: n.y, fx: n.fx, fy: n.fy };
        });
    });

    function dragstarted(event) {
        if (!event.active) simulation.alphaTarget(0.2).restart();
        event.subject.fx = event.subject.x;
        event.subject.fy = event.subject.y;
    }

    function dragged(event) {
        event.subject.fx = event.x;
        event.subject.fy = event.y;
    }

    function dragended(event) {
        if (!event.active) simulation.alphaTarget(0);
        // Keep node fixed where user dropped it
        nodePositions[event.subject.id] = {
            x: event.subject.x,
            y: event.subject.y,
            fx: event.subject.x,
            fy: event.subject.y
        };
    }
}

function getNodeColor(node) {
    const type = node.node_type || 'Unknown';

    // Color based on device type (simplified, no TSN-specific)
    const typeColors = {
        'Router': '#d29922',         // Yellow - Router
        'Gateway': '#d29922',        // Yellow - Gateway
        'Switch': '#2ea043',         // Green - Switch
        'Bridge': '#2ea043',         // Green - Bridge
        'AccessPoint': '#a371f7',    // Purple - WiFi AP
        'Host': '#2f81f7',           // Blue - Host/PC
        'EndStation': '#58a6ff',     // Light blue - End device
        'Server': '#bf5af2',         // Purple - Server
        'Unknown': '#8b949e',        // Gray - Unknown
    };

    return typeColors[type] || '#8b949e';
}

function getNodeRadius(node) {
    const packets = (node.packets_sent || 0) + (node.packets_received || 0);
    const base = 10;
    const scaled = Math.min(Math.max(base, Math.log(packets + 1) * 3 + base), 25);

    // Larger for important node types
    const type = node.node_type || 'Unknown';
    if (type === 'Router' || type === 'Gateway' || type === 'Server') {
        return scaled + 5;
    }
    if (type === 'TsnBridge' || type === 'Switch') {
        return scaled + 3;
    }

    return scaled;
}

function setLayout(mode) {
    state.layoutMode = mode;
    nodePositions = {};  // Clear positions for new layout
    lastTopologyHash = '';  // Force re-render
    document.getElementById('btn-layout-force').classList.toggle('active', mode === 'force');
    document.getElementById('btn-layout-radial').classList.toggle('active', mode === 'radial');
    renderTopology();
}

function zoomTopology(factor) {
    if (svg && zoom) {
        svg.transition().duration(300).call(zoom.scaleBy, factor);
    }
}

function fitTopology() {
    if (svg && zoom) {
        svg.transition().duration(300).call(zoom.transform, d3.zoomIdentity);
    }
}

// Network Scanning
async function scanNetwork() {
    const btn = document.getElementById('btn-scan-network');
    btn.disabled = true;
    btn.textContent = 'Scanning...';

    try {
        const data = await apiCall('/api/topology/scan', {
            method: 'POST',
            body: JSON.stringify({ quick: true }),
        });

        if (data && data.success) {
            const result = data.data;
            showNotification(`Scan complete: Found ${result.hosts_found} hosts in ${result.scan_duration_ms}ms`, 'success');
            // Refresh topology to include new hosts
            await refreshTopology();
        } else {
            showNotification(`Scan failed: ${data?.error || 'Unknown error'}`, 'error');
        }
    } catch (err) {
        showNotification(`Scan error: ${err.message}`, 'error');
    } finally {
        btn.disabled = false;
        btn.textContent = 'Scan Network';
    }
}

function updateTopologyStats() {
    const topology = state.topology;

    const nodeCountEl = document.getElementById('topo-node-count');
    const linkCountEl = document.getElementById('topo-link-count');

    if (nodeCountEl) nodeCountEl.textContent = topology.nodes?.length || 0;
    if (linkCountEl) linkCountEl.textContent = topology.links?.length || 0;
}

// Interface Selection
async function showInterfaceModal() {
    const data = await apiCall('/api/interfaces');
    if (data && data.success) {
        const list = document.getElementById('interface-list');
        list.innerHTML = data.data.map(iface => `
            <div class="interface-item" onclick="selectInterface('${iface.name}')">
                <span class="interface-name">${iface.name}</span>
                <span class="interface-desc">${iface.description || ''}</span>
                ${iface.addresses?.length ? `<span class="interface-addr">${iface.addresses.join(', ')}</span>` : ''}
            </div>
        `).join('');
        document.getElementById('interface-modal').style.display = 'flex';
    }
}

function hideInterfaceModal() {
    document.getElementById('interface-modal').style.display = 'none';
}

async function selectInterface(name) {
    const result = await apiCall('/api/interface/set', 'POST', { interface: name });
    if (result && result.success) {
        document.getElementById('interface-name').textContent = name;
    }
    hideInterfaceModal();
}

// File Operations
async function savePcap() {
    if (state.packets.length === 0) {
        showNotification('No packets to save', 'error');
        return;
    }

    try {
        const response = await fetch('/api/pcap/download', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' }
        });

        if (response.ok) {
            const blob = await response.blob();
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
            a.href = url;
            a.download = `capture_${timestamp}.pcap`;
            document.body.appendChild(a);
            a.click();
            document.body.removeChild(a);
            URL.revokeObjectURL(url);
            showNotification(`${state.packets.length} packets saved`, 'success');
        } else {
            showNotification('PCAP save failed', 'error');
        }
    } catch (error) {
        console.error('Save error:', error);
        showNotification('PCAP save failed', 'error');
    }
}

function loadPcap() {
    document.getElementById('pcap-file-input').click();
}

async function handlePcapFileSelect(event) {
    const file = event.target.files[0];
    if (!file) return;

    const formData = new FormData();
    formData.append('file', file);

    try {
        showNotification('Loading file...', 'info');
        const response = await fetch('/api/pcap/upload', {
            method: 'POST',
            body: formData
        });

        if (response.ok) {
            const result = await response.json();
            if (result.success) {
                clearAll();
                document.getElementById('capture-file').textContent = file.name;
                await loadPackets();
                showNotification(`${result.data.packets_loaded} packets loaded`, 'success');
            }
        } else {
            showNotification('PCAP load failed', 'error');
        }
    } catch (error) {
        console.error('Load error:', error);
        showNotification('PCAP load failed', 'error');
    }

    // Reset file input
    event.target.value = '';
}

function showNotification(message, type = 'info') {
    // Remove existing notification
    const existing = document.querySelector('.notification');
    if (existing) existing.remove();

    const notification = document.createElement('div');
    notification.className = `notification notification-${type}`;
    notification.textContent = message;
    document.body.appendChild(notification);

    // Auto remove after 3 seconds
    setTimeout(() => notification.remove(), 3000);
}

function exportCSV() {
    if (state.packets.length === 0) {
        alert('No packets to export');
        return;
    }

    const headers = ['No', 'Time', 'Source', 'Destination', 'Protocol', 'Length', 'Info'];
    const rows = state.packets.map(p => [
        p.id,
        new Date(p.timestamp).toISOString(),
        p.info.src_ip || p.info.src_mac,
        p.info.dst_ip || p.info.dst_mac,
        p.info.protocol || p.info.ethertype_name,
        p.length,
        getPacketInfo(p).replace(/,/g, ';')
    ]);

    const csv = [headers.join(','), ...rows.map(r => r.join(','))].join('\n');
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
    const link = document.createElement('a');
    link.href = URL.createObjectURL(blob);
    link.download = `packets_${Date.now()}.csv`;
    link.click();
}

// Clear All
function clearAll() {
    state.packets = [];
    state.filteredPackets = [];
    state.hosts.clear();
    state.protocols.clear();
    state.conversations.clear();
    state.stats = { packets_captured: 0, bytes_captured: 0, start_time: null };
    state.selectedPacket = null;

    // Reset pagination
    state.currentPage = 1;
    state.totalPages = 1;

    document.getElementById('packet-tbody').innerHTML = '';
    updateCounters();
    updatePaginationUI();
    updateAllCharts();
    renderHostsList();

    document.getElementById('capture-file').textContent = 'No capture file';
    document.getElementById('selected-packet-info').textContent = '-';

    // Reset detail panel
    document.getElementById('detail-placeholder').style.display = 'flex';
    document.getElementById('detail-content').style.display = 'none';
}

// Utility
function formatBytes(bytes) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

// Polling
function startPolling() {
    // Refresh topology periodically if auto-refresh is enabled (throttled to 30s)
    setInterval(() => {
        if (document.getElementById('auto-refresh').checked && state.isCapturing) {
            refreshTopology();
        }
    }, 30000);

    // Update charts periodically
    setInterval(updateAllCharts, 3000);
}

// Keyboard shortcuts
document.addEventListener('keydown', (e) => {
    // Ignore if typing in input
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'SELECT') {
        if (e.key === 'Escape') {
            e.target.blur();
        }
        return;
    }

    switch (e.key) {
        case ' ':  // Space - toggle capture
            e.preventDefault();
            if (state.isCapturing) {
                stopCapture();
            } else {
                startCapture();
            }
            break;
        case 'c':  // C - clear
        case 'C':
            if (!e.ctrlKey && !e.metaKey) {
                clearAll();
            }
            break;
        case '1':  // Tab shortcuts
            switchTab('topology');
            break;
        case '2':
            switchTab('stats');
            break;
        case '3':
            switchTab('hosts');
            break;
        case '4':
            switchTab('detail');
            break;
        case '5':
            switchTab('latency');
            break;
        case '6':
            switchTab('throughput');
            break;
        case 'ArrowUp':  // Navigate packets
            e.preventDefault();
            navigatePacket(-1);
            break;
        case 'ArrowDown':
            e.preventDefault();
            navigatePacket(1);
            break;
        case 'PageUp':  // Navigate pages
            e.preventDefault();
            goToPage(state.currentPage - 1);
            break;
        case 'PageDown':
            e.preventDefault();
            goToPage(state.currentPage + 1);
            break;
        case 'Home':
            if (e.ctrlKey) {
                e.preventDefault();
                goToPage(1);
            }
            break;
        case 'End':
            if (e.ctrlKey) {
                e.preventDefault();
                goToPage(state.totalPages);
            }
            break;
        case 'f':  // Focus filter
        case 'F':
            if (!e.ctrlKey && !e.metaKey) {
                e.preventDefault();
                document.getElementById('packet-filter').focus();
            }
            break;
        case 'Escape':  // Clear filter
            document.getElementById('packet-filter').value = '';
            state.filter = '';
            renderPacketList();
            break;
    }
});

function switchTab(tabName) {
    document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
    document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
    document.querySelector(`[data-tab="${tabName}"]`).classList.add('active');
    document.getElementById(`tab-${tabName}`).classList.add('active');

    if (tabName === 'topology') {
        setTimeout(renderTopology, 100);
    } else if (tabName === 'stats') {
        updateAllCharts();
    } else if (tabName === 'hosts') {
        renderHostsList();
    } else if (tabName === 'latency') {
        if (state.charts.latency) {
            setTimeout(() => state.charts.latency.resize(), 100);
        }
    } else if (tabName === 'throughput') {
        if (state.charts.throughput) {
            setTimeout(() => state.charts.throughput.resize(), 100);
        }
    }
}

function navigatePacket(direction) {
    const packets = state.filteredPackets;
    if (packets.length === 0) return;

    let currentIndex = -1;
    if (state.selectedPacket) {
        currentIndex = packets.findIndex(p => p.id === state.selectedPacket.id);
    }

    let newIndex = currentIndex + direction;
    if (newIndex < 0) newIndex = 0;
    if (newIndex >= packets.length) newIndex = packets.length - 1;

    const packet = packets[newIndex];
    selectPacket(packet);

    // Check if packet is on current page
    const pageForPacket = Math.floor(newIndex / state.pageSize) + 1;
    if (pageForPacket !== state.currentPage) {
        goToPage(pageForPacket);
    }

    // Scroll selected row into view
    const row = document.querySelector(`#packet-tbody tr[data-id="${packet.id}"]`);
    if (row) {
        row.scrollIntoView({ block: 'nearest' });
    }
}

// Global functions for onclick handlers
window.filterByHost = filterByHost;
window.selectInterface = selectInterface;
window.selectHost = selectHost;

// ============================================
// Column Resize Functionality
// ============================================

function initializeColumnResize() {
    const table = document.getElementById('packet-table');
    if (!table) return;

    const headers = table.querySelectorAll('th.resizable');
    let isResizing = false;
    let currentTh = null;
    let startX = 0;
    let startWidth = 0;

    // Load saved column widths
    const savedWidths = localStorage.getItem('columnWidths');
    if (savedWidths) {
        try {
            const widths = JSON.parse(savedWidths);
            headers.forEach((th, index) => {
                if (widths[index]) {
                    th.style.width = widths[index] + 'px';
                }
            });
        } catch (e) {
            console.warn('Failed to load column widths');
        }
    }

    headers.forEach(th => {
        const handle = th.querySelector('.resize-handle');
        if (!handle) return;

        handle.addEventListener('mousedown', (e) => {
            isResizing = true;
            currentTh = th;
            startX = e.pageX;
            startWidth = th.offsetWidth;
            th.classList.add('resizing');
            handle.classList.add('active');
            document.body.classList.add('col-resizing');
            e.preventDefault();
        });
    });

    document.addEventListener('mousemove', (e) => {
        if (!isResizing) return;
        const diff = e.pageX - startX;
        const newWidth = Math.max(40, startWidth + diff);
        currentTh.style.width = newWidth + 'px';
    });

    document.addEventListener('mouseup', () => {
        if (!isResizing) return;
        isResizing = false;
        if (currentTh) {
            currentTh.classList.remove('resizing');
            const handle = currentTh.querySelector('.resize-handle');
            if (handle) handle.classList.remove('active');
        }
        document.body.classList.remove('col-resizing');

        // Save column widths
        const widths = [];
        headers.forEach(th => {
            widths.push(th.offsetWidth);
        });
        localStorage.setItem('columnWidths', JSON.stringify(widths));

        currentTh = null;
    });
}

// ============================================
// Test Charts (Latency & Throughput)
// ============================================

function initializeTestCharts() {
    const chartOptions = {
        responsive: true,
        maintainAspectRatio: false,
        animation: { duration: 300 },
        plugins: {
            legend: {
                labels: { color: '#e6edf3', font: { size: 11 } }
            }
        },
        scales: {
            x: {
                grid: { color: '#30363d' },
                ticks: { color: '#8b949e' }
            },
            y: {
                grid: { color: '#30363d' },
                ticks: { color: '#8b949e' },
                beginAtZero: true
            }
        }
    };

    // Latency Chart
    const latencyCtx = document.getElementById('latency-chart');
    if (latencyCtx) {
        state.charts.latency = new Chart(latencyCtx.getContext('2d'), {
            type: 'line',
            data: {
                labels: [],
                datasets: [{
                    label: 'RTT (ms)',
                    data: [],
                    borderColor: '#0a84ff',
                    backgroundColor: 'rgba(10, 132, 255, 0.1)',
                    tension: 0.3,
                    fill: true,
                    pointRadius: 4,
                    pointHoverRadius: 6
                }]
            },
            options: {
                ...chartOptions,
                plugins: {
                    ...chartOptions.plugins,
                    title: {
                        display: false
                    }
                }
            }
        });
    }

    // Throughput Chart
    const throughputCtx = document.getElementById('throughput-chart');
    if (throughputCtx) {
        state.charts.throughput = new Chart(throughputCtx.getContext('2d'), {
            type: 'line',
            data: {
                labels: [],
                datasets: [{
                    label: 'Mbps',
                    data: [],
                    borderColor: '#30d158',
                    backgroundColor: 'rgba(48, 209, 88, 0.1)',
                    tension: 0.3,
                    fill: true,
                    pointRadius: 4,
                    pointHoverRadius: 6
                }]
            },
            options: {
                ...chartOptions,
                plugins: {
                    ...chartOptions.plugins,
                    title: {
                        display: false
                    }
                }
            }
        });
    }
}

// ============================================
// Tester Functions
// ============================================

// Ping/Latency Test with real-time SSE streaming
async function startPingTest() {
    const target = document.getElementById('ping-target').value.trim();
    const count = parseInt(document.getElementById('ping-count').value) || 10;
    const interval = parseInt(document.getElementById('ping-interval').value) || 1000;

    if (!target) {
        showNotification('Please enter a target host', 'error');
        return;
    }

    const btn = document.getElementById('btn-ping-start');
    btn.disabled = true;
    btn.innerHTML = '<span class="btn-spinner"></span> Testing...';

    // Reset stats and chart
    document.getElementById('ping-min').textContent = '...';
    document.getElementById('ping-avg').textContent = '...';
    document.getElementById('ping-max').textContent = '...';
    document.getElementById('ping-loss').textContent = '...';

    // Initialize chart with empty data
    if (state.charts.latency) {
        state.charts.latency.data.labels = [];
        state.charts.latency.data.datasets[0].data = [];
        state.charts.latency.update();
    }

    const pingResults = [];

    try {
        // Use SSE for real-time updates
        const url = `/api/test/ping/stream?target=${encodeURIComponent(target)}&count=${count}&interval=${interval}`;
        const eventSource = new EventSource(url);

        eventSource.addEventListener('ping', (e) => {
            const data = JSON.parse(e.data);
            pingResults.push(data);

            // Update chart in real-time
            if (state.charts.latency) {
                state.charts.latency.data.labels.push(`#${data.seq + 1}`);
                state.charts.latency.data.datasets[0].data.push(data.success ? data.rtt_ms : null);
                state.charts.latency.update('none');  // No animation for smooth updates
            }
        });

        eventSource.addEventListener('complete', (e) => {
            const data = JSON.parse(e.data);
            eventSource.close();

            // Update final stats
            if (data.stats) {
                document.getElementById('ping-min').textContent = data.stats.min_ms?.toFixed(2) || '-';
                document.getElementById('ping-avg').textContent = data.stats.avg_ms?.toFixed(2) || '-';
                document.getElementById('ping-max').textContent = data.stats.max_ms?.toFixed(2) || '-';
                document.getElementById('ping-loss').textContent = (data.stats.loss_percent?.toFixed(1) || '0') + '%';
            }

            btn.disabled = false;
            btn.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg> Start Test';
            showNotification(`Ping complete: ${pingResults.filter(r => r.success).length}/${pingResults.length} successful`, 'success');
        });

        eventSource.addEventListener('error', (e) => {
            if (e.data) {
                showNotification(`Error: ${e.data}`, 'error');
            }
            eventSource.close();
            btn.disabled = false;
            btn.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg> Start Test';
            resetPingStats();
        });

        eventSource.onerror = () => {
            eventSource.close();
            btn.disabled = false;
            btn.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg> Start Test';
            if (pingResults.length === 0) {
                showNotification('Connection failed', 'error');
                resetPingStats();
            }
        };

    } catch (e) {
        showNotification(`Error: ${e.message}`, 'error');
        resetPingStats();
        btn.disabled = false;
        btn.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg> Start Test';
    }
}

function resetPingStats() {
    document.getElementById('ping-min').textContent = '-';
    document.getElementById('ping-avg').textContent = '-';
    document.getElementById('ping-max').textContent = '-';
    document.getElementById('ping-loss').textContent = '-';
}

function renderPingResults(data) {
    if (!data || !data.results) {
        showNotification('No results received', 'error');
        return;
    }

    // Update stat boxes
    if (data.stats) {
        document.getElementById('ping-min').textContent = data.stats.min_ms?.toFixed(2) || '-';
        document.getElementById('ping-avg').textContent = data.stats.avg_ms?.toFixed(2) || '-';
        document.getElementById('ping-max').textContent = data.stats.max_ms?.toFixed(2) || '-';
        document.getElementById('ping-loss').textContent = (data.stats.loss_percent?.toFixed(1) || '0') + '%';
    }

    // Update chart
    if (state.charts.latency) {
        const labels = data.results.map((_, i) => `#${i + 1}`);
        const values = data.results.map(r => r.success ? r.rtt_ms : null);

        state.charts.latency.data.labels = labels;
        state.charts.latency.data.datasets[0].data = values;
        state.charts.latency.update();
    }

    showNotification(`Ping complete: ${data.results.filter(r => r.success).length}/${data.results.length} successful`, 'success');
}

// Throughput Test with real-time SSE streaming
async function startThroughputTest() {
    const target = document.getElementById('throughput-target').value.trim();
    const duration = parseInt(document.getElementById('throughput-duration').value) || 10;
    const bandwidth = parseInt(document.getElementById('throughput-bandwidth').value) || 100;

    if (!target) {
        showNotification('Please enter a target host', 'error');
        return;
    }

    const btn = document.getElementById('btn-throughput-start');
    btn.disabled = true;
    btn.innerHTML = '<span class="btn-spinner"></span> Testing...';

    // Reset stats and chart
    document.getElementById('tp-bandwidth').textContent = '...';
    document.getElementById('tp-packets').textContent = '...';
    document.getElementById('tp-jitter').textContent = '-';
    document.getElementById('tp-loss').textContent = '-';

    // Initialize chart with empty data
    if (state.charts.throughput) {
        state.charts.throughput.data.labels = [];
        state.charts.throughput.data.datasets[0].data = [];
        state.charts.throughput.update();
    }

    try {
        // Use SSE for real-time updates
        const url = `/api/test/throughput/stream?target=${encodeURIComponent(target)}&duration=${duration}&bandwidth=${bandwidth}`;
        const eventSource = new EventSource(url);

        eventSource.addEventListener('throughput', (e) => {
            const data = JSON.parse(e.data);

            // Update live stats
            document.getElementById('tp-bandwidth').textContent = data.bandwidth_mbps.toFixed(2) + ' Mbps';
            document.getElementById('tp-packets').textContent = data.total_packets.toLocaleString();

            // Update chart in real-time
            if (state.charts.throughput) {
                state.charts.throughput.data.labels.push(`${data.sec}s`);
                state.charts.throughput.data.datasets[0].data.push(data.bandwidth_mbps);
                state.charts.throughput.update('none');
            }
        });

        eventSource.addEventListener('complete', (e) => {
            const data = JSON.parse(e.data);
            eventSource.close();

            // Update final stats
            document.getElementById('tp-bandwidth').textContent = data.avg_bandwidth_mbps.toFixed(2) + ' Mbps';
            document.getElementById('tp-packets').textContent = data.total_packets.toLocaleString();

            btn.disabled = false;
            btn.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg> Start Test';
            showNotification(`Throughput: ${data.avg_bandwidth_mbps.toFixed(2)} Mbps (${data.total_packets.toLocaleString()} packets)`, 'success');
        });

        eventSource.addEventListener('error', (e) => {
            if (e.data) {
                showNotification(`Error: ${e.data}`, 'error');
            }
            eventSource.close();
            btn.disabled = false;
            btn.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg> Start Test';
            resetThroughputStats();
        });

        eventSource.onerror = () => {
            eventSource.close();
            btn.disabled = false;
            btn.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg> Start Test';
        };

    } catch (e) {
        showNotification(`Error: ${e.message}`, 'error');
        resetThroughputStats();
        btn.disabled = false;
        btn.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg> Start Test';
    }
}

function resetThroughputStats() {
    document.getElementById('tp-bandwidth').textContent = '-';
    document.getElementById('tp-packets').textContent = '-';
    document.getElementById('tp-jitter').textContent = '-';
    document.getElementById('tp-loss').textContent = '-';
}

function renderThroughputResults(data) {
    if (!data) {
        showNotification('No results received', 'error');
        return;
    }

    // Update stat boxes
    document.getElementById('tp-bandwidth').textContent = formatBandwidth(data.bandwidth_bps);
    document.getElementById('tp-packets').textContent = (data.packets_sent || 0).toLocaleString();
    document.getElementById('tp-jitter').textContent = data.jitter_ms?.toFixed(3) || '-';
    document.getElementById('tp-loss').textContent = (data.loss_percent?.toFixed(2) || '0') + '%';

    // Update chart with interval data if available
    if (state.charts.throughput && data.intervals) {
        const labels = data.intervals.map((_, i) => `${i + 1}s`);
        const values = data.intervals.map(v => v.bandwidth_mbps || 0);

        state.charts.throughput.data.labels = labels;
        state.charts.throughput.data.datasets[0].data = values;
        state.charts.throughput.update();
    } else if (state.charts.throughput) {
        // Single data point
        const mbps = data.bandwidth_bps ? (data.bandwidth_bps / 1e6).toFixed(2) : 0;
        state.charts.throughput.data.labels = ['Result'];
        state.charts.throughput.data.datasets[0].data = [parseFloat(mbps)];
        state.charts.throughput.update();
    }

    showNotification(`Throughput: ${formatBandwidth(data.bandwidth_bps)}`, 'success');
}

function formatBandwidth(bps) {
    if (!bps) return '-';
    if (bps >= 1e9) return (bps / 1e9).toFixed(2) + ' Gbps';
    if (bps >= 1e6) return (bps / 1e6).toFixed(2) + ' Mbps';
    if (bps >= 1e3) return (bps / 1e3).toFixed(2) + ' Kbps';
    return bps + ' bps';
}

// TSN Configuration
async function applyCbsConfig() {
    const iface = document.getElementById('cbs-interface').value;
    const tc = parseInt(document.getElementById('cbs-tc').value);
    const idleSlope = parseInt(document.getElementById('cbs-idle-slope').value);
    const sendSlope = parseInt(document.getElementById('cbs-send-slope').value);

    try {
        const result = await apiCall('/api/tsn/cbs', 'POST', {
            interface: iface,
            traffic_class: tc,
            idle_slope: idleSlope,
            send_slope: sendSlope
        });

        if (result && result.success) {
            showNotification('CBS configuration applied', 'success');
        } else {
            showNotification('Failed to apply CBS: ' + (result?.error || 'Unknown error'), 'error');
        }
    } catch (e) {
        showNotification('Failed to apply CBS: ' + e.message, 'error');
    }
}

async function applyTasConfig() {
    const cycleTime = parseInt(document.getElementById('tas-cycle').value);
    const baseTime = document.getElementById('tas-basetime').value;
    const gclText = document.getElementById('tas-gcl').value;

    // Parse GCL
    const gcl = gclText.split('\n').filter(l => l.trim()).map(line => {
        const parts = line.split(':');
        return {
            tc: parseInt(parts[0]),
            gate_state: parseInt(parts[1]),
            interval: parseInt(parts[2])
        };
    });

    try {
        const result = await apiCall('/api/tsn/tas', 'POST', {
            cycle_time: cycleTime,
            base_time: baseTime,
            gate_control_list: gcl
        });

        if (result && result.success) {
            showNotification('TAS configuration applied', 'success');
        } else {
            showNotification('Failed to apply TAS: ' + (result?.error || 'Unknown error'), 'error');
        }
    } catch (e) {
        showNotification('Failed to apply TAS: ' + e.message, 'error');
    }
}
