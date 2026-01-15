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
    // Pagination
    currentPage: 1,
    pageSize: 100,
    totalPages: 1,
};

// Initialize Application
document.addEventListener('DOMContentLoaded', () => {
    initializeUI();
    initializeCharts();
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
    document.getElementById('btn-layout-force').addEventListener('click', () => setLayout('force'));
    document.getElementById('btn-layout-radial').addEventListener('click', () => setLayout('radial'));
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
        statusEl.textContent = '캡처 중';
        statusEl.className = 'status status-capturing';
        startBtn.disabled = true;
        stopBtn.disabled = false;
    } else {
        statusEl.textContent = '정지';
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
        text.textContent = '연결됨';
    } else {
        dot.className = 'status-dot';
        text.textContent = '연결 끊김';
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

// Packet Processing
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
        // Apply filter and update pagination
        if (matchesFilter(packet, state.filter)) {
            state.filteredPackets.push(packet);

            // Update pagination state
            const newTotalPages = Math.max(1, Math.ceil(state.filteredPackets.length / state.pageSize));

            // If on last page, append the new packet row
            if (state.currentPage === state.totalPages) {
                // Check if we need to move to new page
                const currentPagePackets = state.filteredPackets.length - (state.currentPage - 1) * state.pageSize;
                if (currentPagePackets <= state.pageSize) {
                    appendPacketRow(packet, true);
                } else {
                    // Page is full, update total pages
                    state.totalPages = newTotalPages;
                }
            }

            state.totalPages = newTotalPages;
            updatePaginationUI();
        }
        updateCounters();
    }
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
    document.getElementById('packet-count').textContent = `${state.stats.packets_captured.toLocaleString()} 패킷`;
    document.getElementById('byte-count').textContent = formatBytes(state.stats.bytes_captured);
}

// Packet List Rendering
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

    displayPackets.forEach(packet => appendPacketRow(packet, false));

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

function appendPacketRow(packet, live = false) {
    const tbody = document.getElementById('packet-tbody');
    const row = document.createElement('tr');

    // Protocol-based coloring
    row.className = getProtocolClass(packet);
    row.dataset.id = packet.id;
    row.onclick = () => selectPacket(packet);

    const time = formatTime(packet.timestamp);
    const info = getPacketInfo(packet);

    row.innerHTML = `
        <td>${packet.id}</td>
        <td>${time}</td>
        <td>${packet.info.src_ip || packet.info.src_mac}</td>
        <td>${packet.info.dst_ip || packet.info.dst_mac}</td>
        <td><span class="proto-badge proto-${(packet.info.protocol || packet.info.ethertype_name || '').toLowerCase()}">${packet.info.protocol || packet.info.ethertype_name || '-'}</span></td>
        <td>${packet.length}</td>
        <td class="info-cell">${info}</td>
    `;

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
        `패킷 #${packet.id} | ${packet.info.protocol || packet.info.ethertype_name} | ${packet.length} bytes`;

    // Prepare detail content (will show when detail tab is clicked)
    document.getElementById('detail-placeholder').style.display = 'none';
    document.getElementById('detail-content').style.display = 'block';

    // Fill frame info
    const frameInfo = document.getElementById('frame-info');
    frameInfo.innerHTML = `
        <div class="detail-row"><span>패킷 번호:</span><span>${packet.id}</span></div>
        <div class="detail-row"><span>캡처 시간:</span><span>${new Date(packet.timestamp).toISOString()}</span></div>
        <div class="detail-row"><span>패킷 길이:</span><span>${packet.length} bytes</span></div>
    `;

    // Fill ethernet info
    const ethInfo = document.getElementById('eth-info');
    ethInfo.innerHTML = `
        <div class="detail-row"><span>출발지 MAC:</span><span>${packet.info.src_mac}</span></div>
        <div class="detail-row"><span>목적지 MAC:</span><span>${packet.info.dst_mac}</span></div>
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
            <div class="detail-row"><span>출발지 IP:</span><span>${packet.info.src_ip}</span></div>
            <div class="detail-row"><span>목적지 IP:</span><span>${packet.info.dst_ip}</span></div>
            ${packet.info.ttl ? `<div class="detail-row"><span>TTL:</span><span>${packet.info.ttl}</span></div>` : ''}
            ${packet.info.ip_protocol ? `<div class="detail-row"><span>프로토콜:</span><span>${packet.info.ip_protocol}</span></div>` : ''}
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
            <div class="detail-row"><span>프로토콜:</span><span>${packet.info.protocol}</span></div>
            <div class="detail-row"><span>출발지 포트:</span><span>${packet.info.src_port}</span></div>
            <div class="detail-row"><span>목적지 포트:</span><span>${packet.info.dst_port}</span></div>
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
        hexDump.textContent = '데이터 없음';
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
                legend: { position: 'right', labels: { color: '#e6edf3' } }
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
                    label: '패킷/초',
                    data: [],
                    borderColor: '#2f81f7',
                    backgroundColor: 'rgba(47, 129, 247, 0.1)',
                    tension: 0.4,
                    fill: true,
                    yAxisID: 'y'
                },
                {
                    label: 'KB/초',
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
                label: '패킷 수',
                data: [],
                backgroundColor: '#2f81f7'
            }]
        },
        options: {
            ...chartOptions,
            indexAxis: 'y',
            scales: {
                x: { grid: { color: '#30363d' }, ticks: { color: '#8b949e' }, beginAtZero: true },
                y: { grid: { color: '#30363d' }, ticks: { color: '#e6edf3', font: { size: 10 } } }
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
                label: '패킷 수',
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
            i === trafficHistory.length - 1 ? '현재' : `-${trafficHistory.length - 1 - i}초`
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

    container.innerHTML = hosts.slice(0, 100).map(host => `
        <div class="host-card" onclick="filterByHost('${host.ip}')">
            <div class="host-header">
                <span class="host-ip">${host.ip}</span>
                <span class="host-type">${getHostType(host)}</span>
            </div>
            <div class="host-mac">${host.mac || '알 수 없음'}</div>
            <div class="host-stats">
                <div class="stat-item">
                    <span class="stat-label">송신</span>
                    <span class="stat-value">${host.packets_sent.toLocaleString()} pkts / ${formatBytes(host.bytes_sent)}</span>
                </div>
                <div class="stat-item">
                    <span class="stat-label">수신</span>
                    <span class="stat-value">${host.packets_recv.toLocaleString()} pkts / ${formatBytes(host.bytes_recv)}</span>
                </div>
                <div class="stat-item">
                    <span class="stat-label">프로토콜</span>
                    <span class="stat-value">${[...host.protocols].slice(0, 5).join(', ') || '-'}</span>
                </div>
                <div class="stat-item">
                    <span class="stat-label">포트</span>
                    <span class="stat-value">${[...host.ports].slice(0, 8).join(', ') || '-'}</span>
                </div>
            </div>
            <div class="host-time">
                마지막 활동: ${formatTimeAgo(host.last_seen)}
            </div>
        </div>
    `).join('');
}

function getHostType(host) {
    const ip = host.ip;
    if (ip.startsWith('192.168.') || ip.startsWith('10.') || ip.startsWith('172.16.')) {
        if (ip.endsWith('.1') || ip.endsWith('.254')) return '게이트웨이';
        return '로컬';
    }
    if (ip === '255.255.255.255' || ip.endsWith('.255')) return '브로드캐스트';
    if (ip.startsWith('224.') || ip.startsWith('239.')) return '멀티캐스트';
    return '원격';
}

function filterByHost(ip) {
    document.getElementById('packet-filter').value = `ip.addr==${ip}`;
    applyFilter();
    // Switch to packet list panel (you can implement tab switching here if needed)
}

function formatTimeAgo(timestamp) {
    const diff = Date.now() - timestamp;
    if (diff < 1000) return '방금 전';
    if (diff < 60000) return `${Math.floor(diff / 1000)}초 전`;
    if (diff < 3600000) return `${Math.floor(diff / 60000)}분 전`;
    return `${Math.floor(diff / 3600000)}시간 전`;
}

// Topology
let svg, simulation, zoom;

async function refreshTopology() {
    const data = await apiCall('/api/topology');
    if (data && data.success) {
        state.topology = data.data;
        renderTopology();
    }
}

function renderTopology() {
    const container = document.getElementById('topology-graph');
    container.innerHTML = '';

    const width = container.clientWidth || 600;
    const height = container.clientHeight || 400;

    svg = d3.select('#topology-graph')
        .append('svg')
        .attr('width', width)
        .attr('height', height);

    // Add zoom behavior
    zoom = d3.zoom()
        .scaleExtent([0.1, 4])
        .on('zoom', (event) => {
            g.attr('transform', event.transform);
        });

    svg.call(zoom);

    const g = svg.append('g');

    // Arrow marker for directed links
    svg.append('defs').append('marker')
        .attr('id', 'arrowhead')
        .attr('viewBox', '-0 -5 10 10')
        .attr('refX', 20)
        .attr('refY', 0)
        .attr('orient', 'auto')
        .attr('markerWidth', 6)
        .attr('markerHeight', 6)
        .append('path')
        .attr('d', 'M 0,-5 L 10,0 L 0,5')
        .attr('fill', '#2f81f7');

    // Limit nodes to top 50 by traffic for performance
    let allNodes = state.topology.nodes.map(n => ({ ...n }));
    allNodes.sort((a, b) => (b.packets_sent + b.packets_received) - (a.packets_sent + a.packets_received));
    const nodes = allNodes.slice(0, 50);
    const nodeIds = new Set(nodes.map(n => n.id));

    // Only include links between visible nodes
    const links = state.topology.links
        .filter(l => nodeIds.has(l.source) && nodeIds.has(l.target))
        .map(l => ({ ...l }));

    if (nodes.length === 0) {
        svg.append('text')
            .attr('x', width / 2)
            .attr('y', height / 2)
            .attr('text-anchor', 'middle')
            .attr('fill', '#8b949e')
            .text('캡처를 시작하면 네트워크 노드가 표시됩니다');
        return;
    }

    // Create simulation
    simulation = d3.forceSimulation(nodes)
        .force('link', d3.forceLink(links).id(d => d.id).distance(120))
        .force('charge', d3.forceManyBody().strength(-400))
        .force('center', d3.forceCenter(width / 2, height / 2))
        .force('collision', d3.forceCollide().radius(40));

    // Draw links
    const link = g.append('g')
        .selectAll('line')
        .data(links)
        .join('line')
        .attr('class', 'topology-link')
        .attr('stroke', '#58a6ff')
        .attr('stroke-opacity', 0.6)
        .attr('stroke-width', d => Math.min(Math.log(d.packets + 1) * 0.5 + 1, 4));

    // Draw nodes
    const node = g.append('g')
        .selectAll('g')
        .data(nodes)
        .join('g')
        .attr('class', 'topology-node')
        .call(d3.drag()
            .on('start', dragstarted)
            .on('drag', dragged)
            .on('end', dragended));

    node.append('circle')
        .attr('r', d => getNodeRadius(d))
        .attr('fill', d => getNodeColor(d))
        .attr('stroke', '#30363d')
        .attr('stroke-width', 2);

    // Labels
    if (document.getElementById('show-labels').checked) {
        node.append('text')
            .attr('dx', 15)
            .attr('dy', 4)
            .attr('fill', '#c9d1d9')
            .attr('font-size', '11px')
            .text(d => d.ip_addresses?.[0] || d.mac_address?.substring(0, 8) || d.id.substring(0, 8));
    }

    // Tooltip
    node.append('title')
        .text(d => {
            const ips = d.ip_addresses?.join(', ') || 'Unknown';
            return `IP: ${ips}\nMAC: ${d.mac_address || 'Unknown'}\nType: ${d.node_type || 'Unknown'}\nVendor: ${d.vendor || 'Unknown'}`;
        });

    simulation.on('tick', () => {
        link
            .attr('x1', d => d.source.x)
            .attr('y1', d => d.source.y)
            .attr('x2', d => d.target.x)
            .attr('y2', d => d.target.y);

        node.attr('transform', d => `translate(${d.x},${d.y})`);
    });

    function dragstarted(event) {
        if (!event.active) simulation.alphaTarget(0.3).restart();
        event.subject.fx = event.subject.x;
        event.subject.fy = event.subject.y;
    }

    function dragged(event) {
        event.subject.fx = event.x;
        event.subject.fy = event.y;
    }

    function dragended(event) {
        if (!event.active) simulation.alphaTarget(0);
        event.subject.fx = null;
        event.subject.fy = null;
    }
}

function getNodeColor(node) {
    const type = node.node_type?.toLowerCase() || '';
    const ip = node.ip_addresses?.[0] || '';

    // Gateway detection
    if (ip.endsWith('.1') || ip.endsWith('.254') || type.includes('router') || type.includes('gateway')) {
        return '#d29922'; // Orange for gateway
    }

    // Broadcast/Multicast
    if (ip === '255.255.255.255' || ip.endsWith('.255') || ip.startsWith('224.') || ip.startsWith('239.')) {
        return '#a371f7'; // Purple for broadcast
    }

    // Local
    if (ip.startsWith('192.168.') || ip.startsWith('10.') || ip.startsWith('172.16.')) {
        return '#2f81f7'; // Blue for local
    }

    // Remote
    return '#58a6ff'; // Light blue for remote
}

function getNodeRadius(node) {
    const packets = node.packets_count || 0;
    return Math.min(Math.max(8, Math.log(packets + 1) * 3), 20);
}

function setLayout(mode) {
    state.layoutMode = mode;
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
        showNotification('저장할 패킷이 없습니다', 'error');
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
            showNotification(`${state.packets.length}개 패킷 저장됨`, 'success');
        } else {
            showNotification('PCAP 저장 실패', 'error');
        }
    } catch (error) {
        console.error('Save error:', error);
        showNotification('PCAP 저장 실패', 'error');
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
        showNotification('파일 로딩 중...', 'info');
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
                showNotification(`${result.data.packets_loaded}개 패킷 로드됨`, 'success');
            }
        } else {
            showNotification('PCAP 로드 실패', 'error');
        }
    } catch (error) {
        console.error('Load error:', error);
        showNotification('PCAP 로드 실패', 'error');
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
        alert('내보낼 패킷이 없습니다');
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

    document.getElementById('capture-file').textContent = '캡처 파일 없음';
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
    // Refresh topology periodically if auto-refresh is enabled
    setInterval(() => {
        if (document.getElementById('auto-refresh').checked && state.isCapturing) {
            refreshTopology();
        }
    }, 5000);

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
