// TSN-Map Frontend Application

const API_BASE = '';

// State
const state = {
    packets: [],
    selectedPacket: null,
    isCapturing: false,
    topology: { nodes: [], links: [] },
    stats: {
        packets_captured: 0,
        bytes_captured: 0,
        tsn_packets: 0,
        ptp_packets: 0,
    },
    eventSource: null,
    charts: {},
};

// Initialize application
document.addEventListener('DOMContentLoaded', () => {
    initializeUI();
    initializeCharts();
    loadStatus();
    setupEventListeners();
    startPolling();
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
                refreshTopology();
            }
        });
    });
}

// Event Listeners
function setupEventListeners() {
    document.getElementById('btn-start').addEventListener('click', startCapture);
    document.getElementById('btn-stop').addEventListener('click', stopCapture);
    document.getElementById('btn-clear').addEventListener('click', clearPackets);
    document.getElementById('btn-save').addEventListener('click', savePcap);
    document.getElementById('btn-load').addEventListener('click', loadPcap);
    document.getElementById('btn-refresh-topology').addEventListener('click', refreshTopology);
    document.getElementById('interface-name').addEventListener('click', showInterfaceModal);
    document.getElementById('btn-interface-ok').addEventListener('click', setInterface);
    document.getElementById('btn-interface-cancel').addEventListener('click', hideInterfaceModal);

    document.getElementById('packet-filter').addEventListener('input', filterPackets);
    document.getElementById('protocol-filter').addEventListener('change', filterPackets);
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
        return await response.json();
    } catch (error) {
        console.error('API Error:', error);
        return null;
    }
}

async function loadStatus() {
    const data = await apiCall('/api/status');
    if (data) {
        document.getElementById('interface-name').textContent = data.interface;
        updateCaptureStatus(data.is_capturing);
        document.getElementById('packet-count').textContent = `${data.packets_captured} packets`;
    }
}

async function loadStats() {
    const data = await apiCall('/api/capture/stats');
    if (data) {
        state.stats = data;
        updateStatsDisplay();
    }
}

async function loadPackets() {
    const data = await apiCall('/api/packets?limit=1000');
    if (data && data.packets) {
        state.packets = data.packets;
        renderPacketList();
    }
}

async function startCapture() {
    const result = await apiCall('/api/capture/start', 'POST');
    if (result && result.success) {
        updateCaptureStatus(true);
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
    const connStatus = document.getElementById('connection-status');

    if (capturing) {
        statusEl.textContent = 'Capturing';
        statusEl.className = 'status status-capturing';
        startBtn.disabled = true;
        stopBtn.disabled = false;
        connStatus.textContent = 'Connected';
        connStatus.className = 'connected';
    } else {
        statusEl.textContent = 'Stopped';
        statusEl.className = 'status status-stopped';
        startBtn.disabled = false;
        stopBtn.disabled = true;
        connStatus.textContent = 'Disconnected';
        connStatus.className = '';
    }
}

// Packet Stream (SSE)
function startPacketStream() {
    if (state.eventSource) {
        state.eventSource.close();
    }

    state.eventSource = new EventSource('/api/packets/stream');

    state.eventSource.onmessage = (event) => {
        const packet = JSON.parse(event.data);
        addPacket(packet);
    };

    state.eventSource.onerror = () => {
        console.error('SSE connection error');
    };
}

function stopPacketStream() {
    if (state.eventSource) {
        state.eventSource.close();
        state.eventSource = null;
    }
}

function addPacket(packet) {
    state.packets.push(packet);
    state.stats.packets_captured++;
    state.stats.bytes_captured += packet.length;

    if (packet.info.is_tsn) state.stats.tsn_packets++;
    if (packet.info.is_ptp) state.stats.ptp_packets++;

    // Limit packet list size
    if (state.packets.length > 10000) {
        state.packets.shift();
    }

    appendPacketRow(packet);
    updateStatsDisplay();
    updateCharts(packet);
}

// Packet List Rendering
function renderPacketList() {
    const tbody = document.getElementById('packet-tbody');
    tbody.innerHTML = '';

    state.packets.forEach(packet => appendPacketRow(packet));
}

function appendPacketRow(packet) {
    const tbody = document.getElementById('packet-tbody');
    const row = document.createElement('tr');

    if (packet.info.is_ptp) row.classList.add('ptp');
    else if (packet.info.is_tsn) row.classList.add('tsn');

    row.dataset.id = packet.id;
    row.onclick = () => selectPacket(packet);

    const time = new Date(packet.timestamp).toLocaleTimeString('en-US', {
        hour12: false,
        fractionalSecondDigits: 3
    });

    const info = getPacketInfo(packet);

    row.innerHTML = `
        <td>${packet.id}</td>
        <td>${time}</td>
        <td>${packet.info.src_ip || packet.info.src_mac}</td>
        <td>${packet.info.dst_ip || packet.info.dst_mac}</td>
        <td>${packet.info.protocol || packet.info.ethertype_name}</td>
        <td>${packet.length}</td>
        <td>${info}</td>
    `;

    tbody.appendChild(row);

    // Auto-scroll to bottom
    const packetList = document.querySelector('.packet-list');
    packetList.scrollTop = packetList.scrollHeight;

    document.getElementById('packet-count').textContent = `${state.packets.length} packets`;
}

function getPacketInfo(packet) {
    if (packet.info.is_ptp && packet.tsn_info?.ptp_info) {
        return `PTP ${packet.tsn_info.ptp_info.message_type} seq=${packet.tsn_info.ptp_info.sequence_id}`;
    }
    if (packet.info.vlan_id) {
        return `VLAN ${packet.info.vlan_id} PCP ${packet.info.vlan_pcp}`;
    }
    if (packet.info.src_port && packet.info.dst_port) {
        return `${packet.info.src_port} → ${packet.info.dst_port}`;
    }
    return packet.info.ethertype_name;
}

function selectPacket(packet) {
    state.selectedPacket = packet;

    // Update selection
    document.querySelectorAll('#packet-tbody tr').forEach(row => {
        row.classList.toggle('selected', row.dataset.id == packet.id);
    });

    // Show detail view
    document.getElementById('packet-detail').style.display = 'block';

    // Fill details
    document.getElementById('detail-timestamp').textContent = new Date(packet.timestamp).toISOString();
    document.getElementById('detail-length').textContent = `${packet.length} bytes`;
    document.getElementById('detail-src-mac').textContent = packet.info.src_mac;
    document.getElementById('detail-dst-mac').textContent = packet.info.dst_mac;
    document.getElementById('detail-ethertype').textContent = packet.info.ethertype_name;
    document.getElementById('detail-vlan').textContent = packet.info.vlan_id || '-';
    document.getElementById('detail-pcp').textContent = packet.info.vlan_pcp ?? '-';
    document.getElementById('detail-src-ip').textContent = packet.info.src_ip || '-';
    document.getElementById('detail-dst-ip').textContent = packet.info.dst_ip || '-';
    document.getElementById('detail-protocol').textContent = packet.info.protocol || '-';

    // TSN Info
    if (packet.tsn_info) {
        document.getElementById('tsn-detail').style.display = 'block';
        document.getElementById('tsn-stream-id').textContent = packet.tsn_info.stream_id || '-';
        document.getElementById('tsn-type').textContent = packet.tsn_info.tsn_type;
        document.getElementById('tsn-tc').textContent = packet.tsn_info.traffic_class ?? '-';
    } else {
        document.getElementById('tsn-detail').style.display = 'none';
    }

    // Hex view
    renderHexView(packet.data);
}

function renderHexView(data) {
    const hexView = document.getElementById('hex-view');
    let output = '';

    for (let i = 0; i < data.length && i < 256; i += 16) {
        const offset = i.toString(16).padStart(4, '0');
        const hex = data.slice(i, i + 16).map(b => b.toString(16).padStart(2, '0')).join(' ');
        const ascii = data.slice(i, i + 16).map(b => b >= 32 && b < 127 ? String.fromCharCode(b) : '.').join('');
        output += `${offset}  ${hex.padEnd(47)}  ${ascii}\n`;
    }

    hexView.textContent = output;
}

function filterPackets() {
    const filter = document.getElementById('packet-filter').value.toLowerCase();
    const protocol = document.getElementById('protocol-filter').value.toLowerCase();

    document.querySelectorAll('#packet-tbody tr').forEach(row => {
        const id = parseInt(row.dataset.id);
        const packet = state.packets.find(p => p.id === id);

        if (!packet) return;

        let show = true;

        if (protocol) {
            if (protocol === 'ptp') show = packet.info.is_ptp;
            else if (protocol === 'tsn') show = packet.info.is_tsn;
            else show = packet.info.protocol?.toLowerCase() === protocol;
        }

        if (filter && show) {
            const text = JSON.stringify(packet).toLowerCase();
            show = text.includes(filter);
        }

        row.style.display = show ? '' : 'none';
    });
}

function clearPackets() {
    state.packets = [];
    state.stats = {
        packets_captured: 0,
        bytes_captured: 0,
        tsn_packets: 0,
        ptp_packets: 0,
    };
    document.getElementById('packet-tbody').innerHTML = '';
    document.getElementById('packet-count').textContent = '0 packets';
    updateStatsDisplay();
}

// Stats Display
function updateStatsDisplay() {
    document.getElementById('stat-packets').textContent = state.stats.packets_captured.toLocaleString();
    document.getElementById('stat-bytes').textContent = formatBytes(state.stats.bytes_captured);
    document.getElementById('stat-tsn').textContent = state.stats.tsn_packets.toLocaleString();
    document.getElementById('stat-ptp').textContent = state.stats.ptp_packets.toLocaleString();
    document.getElementById('stat-rate').textContent = `${state.stats.capture_rate?.toFixed(1) || 0} pps`;

    const bw = (state.stats.bytes_captured * 8) / 1000000;
    document.getElementById('stat-bandwidth').textContent = `${bw.toFixed(2)} Mbps`;
}

function formatBytes(bytes) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

// Charts
function initializeCharts() {
    // Traffic Chart
    const trafficCtx = document.getElementById('traffic-chart').getContext('2d');
    state.charts.traffic = new Chart(trafficCtx, {
        type: 'line',
        data: {
            labels: [],
            datasets: [{
                label: 'Packets/s',
                data: [],
                borderColor: '#00d4aa',
                tension: 0.4,
                fill: true,
                backgroundColor: 'rgba(0, 212, 170, 0.1)',
            }]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            scales: {
                y: { beginAtZero: true, grid: { color: '#30363d' } },
                x: { grid: { color: '#30363d' } }
            },
            plugins: { legend: { labels: { color: '#c9d1d9' } } }
        }
    });

    // Protocol Distribution Chart
    const protocolCtx = document.getElementById('protocol-chart').getContext('2d');
    state.charts.protocol = new Chart(protocolCtx, {
        type: 'doughnut',
        data: {
            labels: ['TCP', 'UDP', 'PTP', 'ARP', 'Other'],
            datasets: [{
                data: [0, 0, 0, 0, 0],
                backgroundColor: ['#58a6ff', '#00d4aa', '#a371f7', '#d29922', '#8b949e']
            }]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            plugins: { legend: { labels: { color: '#c9d1d9' }, position: 'right' } }
        }
    });

    // PTP Chart
    const ptpCtx = document.getElementById('ptp-chart').getContext('2d');
    state.charts.ptp = new Chart(ptpCtx, {
        type: 'line',
        data: {
            labels: [],
            datasets: [
                { label: 'Offset (ns)', data: [], borderColor: '#58a6ff', tension: 0.4 },
                { label: 'Delay (ns)', data: [], borderColor: '#d29922', tension: 0.4 }
            ]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            scales: {
                y: { grid: { color: '#30363d' } },
                x: { grid: { color: '#30363d' } }
            },
            plugins: { legend: { labels: { color: '#c9d1d9' } } }
        }
    });
}

const protocolCounts = { tcp: 0, udp: 0, ptp: 0, arp: 0, other: 0 };
let trafficHistory = [];

function updateCharts(packet) {
    // Update protocol counts
    if (packet.info.is_ptp) protocolCounts.ptp++;
    else if (packet.info.protocol === 'TCP') protocolCounts.tcp++;
    else if (packet.info.protocol === 'UDP') protocolCounts.udp++;
    else if (packet.info.ethertype_name === 'ARP') protocolCounts.arp++;
    else protocolCounts.other++;

    state.charts.protocol.data.datasets[0].data = [
        protocolCounts.tcp, protocolCounts.udp, protocolCounts.ptp,
        protocolCounts.arp, protocolCounts.other
    ];
    state.charts.protocol.update('none');

    // Update traffic chart periodically
    const now = new Date();
    trafficHistory.push({ time: now, size: packet.length });

    // Keep only last 60 seconds
    const cutoff = new Date(now - 60000);
    trafficHistory = trafficHistory.filter(h => h.time > cutoff);
}

// Periodic traffic chart update
setInterval(() => {
    if (trafficHistory.length === 0) return;

    const now = new Date();
    const labels = [];
    const data = [];

    for (let i = 59; i >= 0; i--) {
        const t = new Date(now - i * 1000);
        labels.push(t.toLocaleTimeString('en-US', { hour12: false }));

        const count = trafficHistory.filter(h =>
            h.time >= new Date(t - 1000) && h.time < t
        ).length;
        data.push(count);
    }

    state.charts.traffic.data.labels = labels;
    state.charts.traffic.data.datasets[0].data = data;
    state.charts.traffic.update('none');
}, 1000);

// Topology
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

    const width = container.clientWidth;
    const height = container.clientHeight || 400;

    const svg = d3.select('#topology-graph')
        .append('svg')
        .attr('width', width)
        .attr('height', height);

    // Prepare data
    const nodes = state.topology.nodes.map(n => ({
        id: n.id,
        ...n
    }));

    const links = state.topology.links.map(l => ({
        source: l.source,
        target: l.target,
        ...l
    }));

    if (nodes.length === 0) {
        svg.append('text')
            .attr('x', width / 2)
            .attr('y', height / 2)
            .attr('text-anchor', 'middle')
            .attr('fill', '#8b949e')
            .text('No nodes discovered. Start capture to detect network devices.');
        return;
    }

    // Force simulation
    const simulation = d3.forceSimulation(nodes)
        .force('link', d3.forceLink(links).id(d => d.id).distance(100))
        .force('charge', d3.forceManyBody().strength(-300))
        .force('center', d3.forceCenter(width / 2, height / 2));

    // Draw links
    const link = svg.append('g')
        .selectAll('line')
        .data(links)
        .join('line')
        .attr('class', d => d.is_tsn_path ? 'link tsn' : 'link')
        .attr('stroke-width', d => Math.min(Math.log(d.packets + 1), 5));

    // Draw nodes
    const node = svg.append('g')
        .selectAll('g')
        .data(nodes)
        .join('g')
        .attr('class', 'node')
        .call(d3.drag()
            .on('start', dragstarted)
            .on('drag', dragged)
            .on('end', dragended));

    node.append('circle')
        .attr('r', d => d.tsn_capable ? 12 : 8)
        .attr('fill', d => getNodeColor(d))
        .attr('stroke', d => d.tsn_capable ? '#00d4aa' : '#30363d');

    if (document.getElementById('show-labels').checked) {
        node.append('text')
            .attr('dx', 15)
            .attr('dy', 4)
            .text(d => d.ip_addresses?.[0] || d.mac_address.substring(0, 8));
    }

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
    switch (node.node_type) {
        case 'PtpGrandmaster': return '#a371f7';
        case 'TsnBridge': return '#00d4aa';
        case 'Switch': return '#d29922';
        case 'Router': return '#d29922';
        default: return '#58a6ff';
    }
}

// Interface Selection
async function showInterfaceModal() {
    const data = await apiCall('/api/interfaces');
    if (data && data.success) {
        const select = document.getElementById('interface-select');
        select.innerHTML = data.data.map(iface =>
            `<option value="${iface.name}">${iface.name} - ${iface.description || 'No description'}</option>`
        ).join('');
        document.getElementById('interface-modal').style.display = 'flex';
    }
}

function hideInterfaceModal() {
    document.getElementById('interface-modal').style.display = 'none';
}

async function setInterface() {
    const iface = document.getElementById('interface-select').value;
    const result = await apiCall('/api/interface/set', 'POST', { interface: iface });
    if (result && result.success) {
        document.getElementById('interface-name').textContent = iface;
    }
    hideInterfaceModal();
}

// PCAP Save/Load
async function savePcap() {
    const filename = prompt('Enter filename:', `capture_${Date.now()}.pcap`);
    if (!filename) return;

    const result = await apiCall('/api/pcap/save', 'POST', { filename });
    if (result && result.success) {
        alert(`Saved ${result.data.packets_saved} packets to ${filename}`);
    } else {
        alert('Failed to save PCAP file');
    }
}

async function loadPcap() {
    const filename = prompt('Enter PCAP filename to load:');
    if (!filename) return;

    const result = await apiCall('/api/pcap/load', 'POST', { filename });
    if (result && result.success) {
        state.packets = [];
        await loadPackets();
        alert(`Loaded ${result.data.packets_loaded} packets from ${filename}`);
    } else {
        alert('Failed to load PCAP file');
    }
}

// Polling
function startPolling() {
    setInterval(async () => {
        await loadStats();
    }, 2000);
}
