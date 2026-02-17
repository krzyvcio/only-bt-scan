const API_BASE = '/api';
const POLL_INTERVAL = 2000;
const PACKET_POLL_INTERVAL = 1000;

let devices = [];
let packets = [];
let lastPacketId = 0;
let isAutoScrollEnabled = true;
let pollTimers = {};

let devicesPage = 1;
let packetsPage = 1;
let devicesTotalPages = 1;
let packetsTotalPages = 1;
let devicesTotal = 0;
let packetsTotal = 0;

document.addEventListener('DOMContentLoaded', () => {
    initApp();
});

function initApp() {
    moment.locale('pl');
    setupEventListeners();
    loadData();
    startPolling();
    showToast('info', 'Połączono z panelem skanera');
}

function setupEventListeners() {
    const refreshBtn = document.getElementById('refresh-btn');
    if (refreshBtn) {
        refreshBtn.addEventListener('click', () => {
            loadData();
            showToast('info', 'Odświeżanie danych...');
        });
    }

    const telemetryBtn = document.getElementById('telemetry-toggle-btn');
    if (telemetryBtn) {
        telemetryBtn.style.display = 'block';
        telemetryBtn.addEventListener('click', () => {
            const telemetrySection = document.getElementById('telemetry-section-hidden');
            if (telemetrySection) {
                const isHidden = telemetrySection.style.display === 'none';
                telemetrySection.style.display = isHidden ? 'flex' : 'none';
                telemetryBtn.textContent = isHidden ? '📊 Pokaż Telemetrię' : '📊 Ukryj Telemetrię';
                if (isHidden) loadTelemetry();
            }
        });
    }

    const searchInput = document.getElementById('search-input');
    if (searchInput) {
        searchInput.addEventListener('input', (e) => {
            filterDevices(e.target.value);
        });
    }

    const autoScroll = document.getElementById('auto-scroll');
    if (autoScroll) {
        autoScroll.addEventListener('change', (e) => {
            isAutoScrollEnabled = e.target.checked;
        });
    }

    // Tab switching
    document.querySelectorAll('.tab').forEach(tab => {
        tab.addEventListener('click', () => {
            document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
            document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
            tab.classList.add('active');
            document.getElementById('tab-' + tab.dataset.tab).classList.add('active');
            
            // Load data for the active tab
            if (tab.dataset.tab === 'all-packets') {
                loadAllPackets();
            } else if (tab.dataset.tab === 'scan-history') {
                loadScanHistory();
            }
        });
    });

    // Modal close
    const closeModal = document.getElementById('close-modal');
    if (closeModal) {
        closeModal.addEventListener('click', () => {
            document.getElementById('device-history-modal').classList.remove('active');
        });
    }

    // Packet modal close
    const closePacketModal = document.getElementById('close-packet-modal');
    if (closePacketModal) {
        closePacketModal.addEventListener('click', () => {
            document.getElementById('packet-details-modal').classList.remove('active');
        });
    }

    // Close modals on background click
    window.addEventListener('click', (e) => {
        if (e.target.classList.contains('modal')) {
            e.target.classList.remove('active');
        }
    });
}

function startPolling() {
    pollTimers.devices = setInterval(loadDevices, POLL_INTERVAL);
    pollTimers.packets = setInterval(loadPackets, PACKET_POLL_INTERVAL);
    pollTimers.stats = setInterval(loadStats, POLL_INTERVAL);
    pollTimers.telemetry = setInterval(loadTelemetry, POLL_INTERVAL);
    pollTimers.timestamps = setInterval(refreshTimestamps, 60000); // Refresh relative times every minute
}

function stopPolling() {
    Object.values(pollTimers).forEach(timer => clearInterval(timer));
    pollTimers = {};
}

function refreshTimestamps() {
    // Re-render current data to update relative timestamps
    if (devices.length > 0) {
        renderDevices(devices);
    }
    if (packets.length > 0) {
        renderPackets();
    }
}

async function loadData() {
    await Promise.all([loadDevices(), loadPackets(), loadStats()]);
}

async function loadDevices() {
    try {
        const response = await fetch(`${API_BASE}/devices?page=${devicesPage}&page_size=50`);
        if (!response.ok) throw new Error('Failed to fetch devices');
        
        const result = await response.json();
        devices = result.data || result;
        devicesTotalPages = result.total_pages || 1;
        devicesTotal = result.total || 0;
        renderDevices(devices);
        updateDevicesPagination();
    } catch (error) {
        console.error('Error loading devices:', error);
    }
}

async function loadPackets() {
    try {
        const response = await fetch(`${API_BASE}/raw-packets?page=${packetsPage}&page_size=50`);
        if (!response.ok) throw new Error('Failed to fetch packets');
        
        const result = await response.json();
        const newPackets = result.data || result;
        
        if (newPackets.length > 0 && newPackets[0].id !== lastPacketId) {
            const oldLength = packets.length;
            packets = newPackets.slice(0, 100);
            
            if (packets.length > oldLength || packets[0]?.id !== lastPacketId) {
                renderPackets();
                lastPacketId = packets[0]?.id || 0;
            }
        }
        packetsTotalPages = result.total_pages || 1;
        packetsTotal = result.total || 0;
        updatePacketsPagination();
    } catch (error) {
        console.error('Error loading packets:', error);
    }
}

async function loadStats() {
    try {
        const response = await fetch(`${API_BASE}/stats`);
        if (!response.ok) throw new Error('Failed to fetch stats');
        
        const stats = await response.json();
        updateStats(stats);
    } catch (error) {
        console.error('Error loading stats:', error);
    }
}

async function loadTelemetry() {
    try {
        const response = await fetch(`${API_BASE}/telemetry`);
        if (!response.ok) throw new Error('Failed to fetch telemetry');
        
        const telemetry = await response.json();
        updateTelemetry(telemetry);
        
        await loadRssiTelemetry();
    } catch (error) {
        console.error('Error loading telemetry:', error);
    }
}

async function loadRssiTelemetry() {
    try {
        const response = await fetch(`${API_BASE}/rssi-telemetry`);
        if (!response.ok) throw new Error('Failed to fetch RSSI telemetry');
        
        const rssiData = await response.json();
        updateRssiTelemetry(rssiData);
    } catch (error) {
        console.error('Error loading RSSI telemetry:', error);
    }
}

function updateRssiTelemetry(data) {
    const tbody = document.getElementById('rssi-telemetry-tbody');
    if (!tbody) return;
    
    const devices = data.devices || [];
    
    document.getElementById('rssi-tracked-devices').textContent = devices.length;
    
    const approaching = devices.filter(d => d.trend === 'approaching').length;
    const leaving = devices.filter(d => d.trend === 'leaving').length;
    const stable = devices.filter(d => d.trend === 'stable').length;
    
    document.getElementById('rssi-approaching').textContent = approaching;
    document.getElementById('rssi-leaving').textContent = leaving;
    document.getElementById('rssi-stable').textContent = stable;
    
    if (devices.length === 0) {
        tbody.innerHTML = '<tr><td colspan="7" class="loading-msg">No RSSI data yet...</td></tr>';
        return;
    }
    
    tbody.innerHTML = devices.map(d => {
        const trendClass = d.trend === 'approaching' ? 'trend-approaching' : 
                          d.trend === 'leaving' ? 'trend-leaving' : 'trend-stable';
        const motionClass = d.motion === 'still' ? 'motion-still' : 'motion-moving';
        
        return `
            <tr>
                <td style="font-family: var(--font-mono); font-size: 0.8rem;">${d.mac}</td>
                <td style="text-align: center;">${d.rssi.toFixed(1)} dBm</td>
                <td style="text-align: center;"><span class="trend-badge ${trendClass}">${d.trend}</span></td>
                <td style="text-align: center;"><span class="motion-badge ${motionClass}">${d.motion}</span></td>
                <td style="text-align: center;">${d.slope.toFixed(3)}</td>
                <td style="text-align: center;">${d.variance.toFixed(2)}</td>
                <td style="text-align: center;">${(d.confidence * 100).toFixed(0)}%</td>
            </tr>
        `;
    }).join('');
}

function updateTelemetry(telemetry) {
    // Display timestamp
    if (telemetry.timestamp) {
        const date = new Date(telemetry.timestamp);
        const timeStr = date.toLocaleTimeString('en-US', { 
            hour: '2-digit', 
            minute: '2-digit', 
            second: '2-digit',
            hour12: false
        });
        document.getElementById('telem-timestamp').textContent = timeStr;
    }
    
    document.getElementById('telem-total-packets').textContent = formatNumber(telemetry.total_packets || 0);
    document.getElementById('telem-total-devices').textContent = telemetry.total_devices || 0;
    
    const tbody = document.getElementById('telemetry-tbody');
    if (!tbody) return;
    
    if (!telemetry.devices || Object.keys(telemetry.devices).length === 0) {
        tbody.innerHTML = '<tr><td colspan="5" class="loading-msg">No telemetry data yet...</td></tr>';
        return;
    }
    
    const devicesList = Object.entries(telemetry.devices)
        .sort((a, b) => b[1].packet_count - a[1].packet_count)
        .slice(0, 20);
    
    tbody.innerHTML = devicesList.map(([mac, data]) => `
        <tr>
            <td style="font-family: var(--font-mono); font-size: 0.8rem;">${mac}</td>
            <td style="text-align: center; font-weight: 600; color: var(--accent-cyan);">${formatNumber(data.packet_count)}</td>
            <td style="text-align: center;">${data.avg_rssi ? data.avg_rssi.toFixed(1) : '-'} dBm</td>
            <td style="text-align: center;">${data.latencies?.min_ms ? data.latencies.min_ms + ' ms' : '-'}</td>
            <td style="text-align: center;">${data.latencies?.max_ms ? data.latencies.max_ms + ' ms' : '-'}</td>
        </tr>
    `).join('');
}

function updateStats(stats) {
    document.getElementById('total-devices').textContent = stats.total_devices || 0;
    document.getElementById('total-packets').textContent = formatNumber(stats.total_packets || 0);
    document.getElementById('active-devices').textContent = stats.active_devices || 0;
    const totalScansEl = document.getElementById('total-scans');
    if (totalScansEl) {
        totalScansEl.textContent = stats.total_scans || 0;
    }
}

async function loadAllPackets() {
    try {
        const response = await fetch(`${API_BASE}/raw-packets/all`);
        if (!response.ok) throw new Error('Failed to fetch packets');
        
        const packets = await response.json();
        renderAllPackets(packets);
        document.getElementById('all-packet-count').textContent = `${packets.length} packets`;
    } catch (error) {
        console.error('Error loading all packets:', error);
    }
}

function renderAllPackets(packets) {
    const list = document.getElementById('all-packets-list');
    
    if (!packets || packets.length === 0) {
        list.innerHTML = `
            <div class="empty-packets">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M22 12h-4l-3 9L9 3l-3 9H2"/>
                </svg>
                <span>No packets found</span>
            </div>
        `;
        return;
    }

    list.innerHTML = packets.slice(0, 500).map(packet => `
        <div class="packet-row">
            <span class="packet-time">${formatPacketTime(packet.timestamp)}</span>
            <span class="packet-mac">${packet.mac_address.substring(0, 17)}</span>
            <span class="packet-rssi">${packet.rssi}</span>
            <span class="packet-phy">${packet.phy || '-'}</span>
            <span class="packet-channel">${packet.channel || '-'}</span>
            <span class="packet-type">${packet.frame_type || '-'}</span>
            <span class="packet-data">${truncateHex(packet.advertising_data || '', 40)}</span>
        </div>
    `).join('');
}

async function loadScanHistory() {
    try {
        const response = await fetch(`${API_BASE}/scan-history`);
        if (!response.ok) throw new Error('Failed to fetch history');
        
        const history = await response.json();
        renderScanHistory(history);
        document.getElementById('history-count').textContent = `${history.length} entries`;
    } catch (error) {
        console.error('Error loading scan history:', error);
    }
}

function renderScanHistory(history) {
    const tbody = document.getElementById('history-tbody');
    
    if (!history || history.length === 0) {
        tbody.innerHTML = `
            <tr>
                <td colspan="4">
                    <div class="empty-state">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <circle cx="12" cy="12" r="10"/>
                            <polyline points="12 6 12 12 16 14"/>
                        </svg>
                        <span>No history found</span>
                    </div>
                </td>
            </tr>
        `;
        return;
    }

    tbody.innerHTML = history.slice(0, 200).map(entry => `
        <tr>
            <td><span class="history-scan-num">#${entry.scan_number}</span></td>
            <td><span class="history-mac">${entry.mac_address}</span></td>
            <td><span class="history-rssi">${entry.rssi} dBm</span></td>
            <td><span class="history-time">${formatTimestamp(entry.timestamp)}</span></td>
        </tr>
    `).join('');
}

function renderDevices(deviceList) {
    const tbody = document.getElementById('devices-tbody');
    
    if (!deviceList || deviceList.length === 0) {
        tbody.innerHTML = `
            <tr>
                <td colspan="10">
                    <div class="empty-state">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <rect x="2" y="7" width="20" height="14" rx="2" ry="2"/>
                            <path d="M16 21V5a2 2 0 00-2-2h-4a2 2 0 00-2 2v16"/>
                        </svg>
                        <span>Nie znaleziono urządzeń. Skanowanie w toku...</span>
                    </div>
                </td>
            </tr>
        `;
        return;
    }

    // Sort by last_seen descending (most recent first)
    const sortedDevices = [...deviceList].sort((a, b) => {
        const aTime = a.last_seen || 0;
        const bTime = b.last_seen || 0;
        return bTime - aTime;
    });

    tbody.innerHTML = sortedDevices.map(device => {
        const signalClass = getSignalClass(device.rssi);
        const rssiClass = getRssiClass(device.rssi);
        const firstSeen = device.first_seen ? formatTimestamp(device.first_seen) : '-';
        const lastSeen = device.last_seen ? formatTimestamp(device.last_seen) : '-';
        const manufacturer = device.manufacturer_name || '-';
        const securityLevel = device.security_level || '-';
        const isRpa = device.is_rpa ? '<span class="rpa-badge">RPA</span>' : '';
        
        return `
            <tr data-mac="${device.mac_address}">
                <td>
                    <div class="signal-bars">
                        ${getSignalBars(device.rssi)}
                    </div>
                    <span class="rssi-value ${rssiClass}">${device.rssi} dBm</span>
                </td>
                <td>
                    <span class="device-name" title="${device.device_name || 'Brak nazwy'}">
                        ${device.device_name || '<em>Brak nazwy</em>'}
                    </span>
                </td>
                <td>
                    <span class="mac-address">${device.mac_address}</span>
                </td>
                <td>
                    <span class="manufacturer" title="${manufacturer}">${manufacturer}</span>
                </td>
                <td>
                    <span class="mac-type">${device.mac_type || '-'} ${isRpa}</span>
                </td>
                <td>
                    <span class="security-badge ${device.security_level === 'Secure Connections' ? 'secure' : ''}">${securityLevel}</span>
                </td>
                <td>
                    <span class="detection-count">${device.number_of_scan || 1}</span>
                </td>
                <td>
                    <span class="timestamp" title="${firstSeen}">${firstSeen}</span>
                </td>
                <td>
                    <span class="timestamp" title="${lastSeen}">${lastSeen}</span>
                </td>
                <td>
                    <button class="history-btn" onclick="showDeviceHistory('${device.mac_address}')">Wyświetl</button>
                </td>
            </tr>
        `;
    }).join('');
    
    // Initialize DataTables after rendering
    if ($.fn.dataTable.isDataTable('#devices-table')) {
        $('#devices-table').DataTable().destroy();
    }
    
    $('#devices-table').DataTable({
        language: {
            url: 'https://cdn.datatables.net/plug-ins/1.13.7/i18n/pl.json'
        },
        pageLength: 25,
        searching: true,
        ordering: true,
        paging: true,
        info: true
    });
}

function renderPackets() {
    const list = document.getElementById('packets-list');
    const countEl = document.getElementById('packet-count');
    
    if (!packets || packets.length === 0) {
        list.innerHTML = `
            <div class="empty-packets">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M22 12h-4l-3 9L9 3l-3 9H2"/>
                </svg>
                <span>Waiting for packets...</span>
            </div>
        `;
        countEl.textContent = '0 packets';
        return;
    }

    countEl.textContent = `${packets.length} packets`;

    const html = packets.map((packet, index) => `
        <div class="packet-row ${index === 0 ? 'new' : ''}" onclick="showPacketDetails(${index})" style="cursor: pointer;" title="Kliknij aby zobaczyć szczegóły">
            <span class="packet-time">${formatPacketTime(packet.timestamp)}</span>
            <span class="packet-mac">${packet.mac_address ? packet.mac_address.substring(0, 17) : '-'}</span>
            <span class="packet-rssi">${packet.rssi}</span>
            <span class="packet-data">${truncateHex(packet.advertising_data || '', 50)}</span>
        </div>
    `).join('');

    list.innerHTML = html;

    if (isAutoScrollEnabled) {
        list.scrollTop = 0;
    }
}

function filterDevices(query) {
    const filtered = devices.filter(device => {
        const searchTerm = query.toLowerCase();
        return (
            (device.mac_address && device.mac_address.toLowerCase().includes(searchTerm)) ||
            (device.device_name && device.device_name.toLowerCase().includes(searchTerm)) ||
            (device.manufacturer_name && device.manufacturer_name.toLowerCase().includes(searchTerm))
        );
    });
    renderDevices(filtered);
}

function updateDevicesPagination() {
    const pageInfo = document.getElementById('devices-page-info');
    if (pageInfo) {
        pageInfo.textContent = `Page ${devicesPage} of ${devicesTotalPages} (${devicesTotal} devices)`;
    }
    const prevBtn = document.getElementById('devices-prev-btn');
    const nextBtn = document.getElementById('devices-next-btn');
    if (prevBtn) prevBtn.disabled = devicesPage <= 1;
    if (nextBtn) nextBtn.disabled = devicesPage >= devicesTotalPages;
}

function updatePacketsPagination() {
    const pageInfo = document.getElementById('packets-page-info');
    if (pageInfo) {
        pageInfo.textContent = `Page ${packetsPage} of ${packetsTotalPages} (${packetsTotal} packets)`;
    }
    const prevBtn = document.getElementById('packets-prev-btn');
    const nextBtn = document.getElementById('packets-next-btn');
    if (prevBtn) prevBtn.disabled = packetsPage <= 1;
    if (nextBtn) nextBtn.disabled = packetsPage >= packetsTotalPages;
}

function goToDevicesPage(page) {
    if (page >= 1 && page <= devicesTotalPages) {
        devicesPage = page;
        loadDevices();
    }
}

function goToPacketsPage(page) {
    if (page >= 1 && page <= packetsTotalPages) {
        packetsPage = page;
        loadPackets();
    }
}

function getSignalClass(rssi) {
    if (rssi >= -50) return 'excellent';
    if (rssi >= -70) return 'good';
    if (rssi >= -85) return 'fair';
    return 'weak';
}

function getRssiClass(rssi) {
    if (rssi >= -50) return 'rssi-excellent';
    if (rssi >= -70) return 'rssi-good';
    if (rssi >= -85) return 'rssi-fair';
    return 'rssi-weak';
}

function getSignalBars(rssi) {
    const bars = [
        rssi >= -70,
        rssi >= -60,
        rssi >= -50,
        rssi >= -40
    ];
    
    const signalClass = getSignalClass(rssi);
    
    return bars.map((active, i) => `
        <div class="signal-bar ${active ? 'active ' + signalClass : ''}" style="height: ${(i + 1) * 5}px"></div>
    `).join('');
}

function formatTimestamp(timestamp) {
    if (!timestamp) return '-';
    try {
        const m = moment(timestamp);
        const date = new Date(timestamp);
        const day = String(date.getDate()).padStart(2, '0');
        const month = String(date.getMonth() + 1).padStart(2, '0');
        const year = date.getFullYear();
        const hours = String(date.getHours()).padStart(2, '0');
        const minutes = String(date.getMinutes()).padStart(2, '0');
        
        return `${day}.${month}.${year} ${hours}:${minutes} (${m.fromNow()})`;
    } catch {
        return '-';
    }
}

function formatPacketTime(timestamp) {
    if (!timestamp) return '-';
    try {
        const m = moment(timestamp);
        const date = new Date(timestamp);
        const day = String(date.getDate()).padStart(2, '0');
        const month = String(date.getMonth() + 1).padStart(2, '0');
        const year = date.getFullYear();
        const hours = String(date.getHours()).padStart(2, '0');
        const minutes = String(date.getMinutes()).padStart(2, '0');
        const seconds = String(date.getSeconds()).padStart(2, '0');
        
        return `${day}.${month}.${year} ${hours}:${minutes}:${seconds} (${m.fromNow()})`;
    } catch {
        return '-';
    }
}

function truncateHex(hex, maxLength = 60) {
    if (!hex) return '-';
    if (hex.length <= maxLength) return hex;
    return hex.substring(0, maxLength) + '...';
}

function formatNumber(num) {
    if (num >= 1000000) return (num / 1000000).toFixed(1) + 'M';
    if (num >= 1000) return (num / 1000).toFixed(1) + 'K';
    return num.toString();
}

function showToast(type, message) {
    const container = document.getElementById('toast-container');
    const toast = document.createElement('div');
    toast.className = `toast toast-${type}`;
    
    const iconPaths = {
        success: '<path d="M22 11.08V12a10 10 0 11-5.93-9.14"/><path d="M22 4L12 14.01l-3-3"/>',
        error: '<circle cx="12" cy="12" r="10"/><line x1="15" y1="9" x2="9" y2="15"/><line x1="9" y1="9" x2="15" y2="15"/>',
        info: '<circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/>'
    };
    
    toast.innerHTML = `
        <svg class="toast-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            ${iconPaths[type] || iconPaths.info}
        </svg>
        <span class="toast-message">${message}</span>
    `;
    
    container.appendChild(toast);
    
    setTimeout(() => {
        toast.style.opacity = '0';
        toast.style.transform = 'translateX(100%)';
        setTimeout(() => toast.remove(), 300);
    }, 3000);
}

window.addEventListener('beforeunload', () => {
    stopPolling();
});

async function showDeviceHistory(mac) {
    try {
        const response = await fetch(`${API_BASE}/devices/${mac}/history`);
        if (!response.ok) throw new Error('Failed to fetch device history');
        
        const data = await response.json();
        
        const modal = document.getElementById('device-history-modal');
        const modalBody = document.getElementById('modal-body');
        const modalDeviceName = document.getElementById('modal-device-name');
        
        // Add data-device-mac attribute for RSSI trend loading
        modal.setAttribute('data-device-mac', mac);
        
        const device = data.device;
        const scanHistory = data.scan_history || [];
        const packets = data.packet_history || [];
        
        // Update modal title
        modalDeviceName.textContent = `${device.device_name || 'Device'} (${mac})`;
        
        // Reset tab to info
        document.querySelectorAll('.modal-tab-content').forEach(tab => tab.classList.remove('active'));
        document.querySelectorAll('.modal-tab-btn').forEach(btn => btn.classList.remove('active'));
        document.getElementById('info-tab').classList.add('active');
        document.querySelector('.modal-tab-btn').classList.add('active');
        
        modalBody.innerHTML = `
            <div class="modal-section">
                <h4>Device Info</h4>
                <p><strong>MAC:</strong> ${device.mac_address}</p>
                <p><strong>Name:</strong> ${device.device_name || 'Unknown'}</p>
                <p><strong>RSSI:</strong> ${device.rssi} dBm</p>
                <p><strong>Manufacturer:</strong> ${device.manufacturer_name || 'Unknown'}</p>
                <p><strong>First Seen:</strong> ${formatTimestamp(device.first_seen)}</p>
                <p><strong>Last Seen:</strong> ${formatTimestamp(device.last_seen)}</p>
                <p><strong>Number of Scans:</strong> ${device.number_of_scan || 1}</p>
            </div>
            <div class="modal-section">
                <h4>Scan History (${scanHistory.length} entries)</h4>
                ${scanHistory.length > 0 ? `
                    <table class="history-table">
                        <thead>
                            <tr><th>#</th><th>RSSI</th><th>Time</th></tr>
                        </thead>
                        <tbody>
                            ${scanHistory.slice(0, 20).map(h => `
                                <tr>
                                    <td>${h.scan_number}</td>
                                    <td>${h.rssi} dBm</td>
                                    <td>${formatTimestamp(h.scan_timestamp)}</td>
                                </tr>
                            `).join('')}
                        </tbody>
                    </table>
                ` : '<p>No scan history</p>'}
            </div>
            <div class="modal-section">
                <h4>Packets (${packets.length} entries)</h4>
                ${packets.length > 0 ? `
                    <table class="history-table">
                        <thead>
                            <tr><th>Time</th><th>RSSI</th><th>Type</th><th>Data</th></tr>
                        </thead>
                        <tbody>
                            ${packets.slice(0, 20).map(p => `
                                <tr>
                                    <td>${formatPacketTime(p.timestamp)}</td>
                                    <td>${p.rssi}</td>
                                    <td>${p.frame_type || '-'}</td>
                                    <td>${truncateHex(p.advertising_data || '', 30)}</td>
                                </tr>
                            `).join('')}
                        </tbody>
                    </table>
                ` : '<p>No packets captured</p>'}
            </div>
        `;
        
        modal.classList.add('active');
    } catch (error) {
        console.error('Error loading device history:', error);
        showToast('error', 'Failed to load device history');
    }
}

// ═══════════════════════════════════════════════════════════════════
// PACKET DETAILS MODAL & ADVERTISING DATA PARSER
// ═══════════════════════════════════════════════════════════════════

function showPacketDetails(index) {
    if (!packets || !packets[index]) return;
    
    const packet = packets[index];
    const modal = document.getElementById('packet-details-modal');
    const modalBody = document.getElementById('packet-modal-body');
    
    const parsed = parseAdvertisingData(packet.advertising_data || '');
    
    modalBody.innerHTML = `
        <div class="packet-detail-section">
            <h4>📋 Informacje o pakiecie</h4>
            <div class="packet-info-grid">
                <div class="info-item">
                    <span class="info-label">MAC Address:</span>
                    <code class="info-value">${packet.mac_address}</code>
                </div>
                <div class="info-item">
                    <span class="info-label">RSSI:</span>
                    <span class="info-value">${packet.rssi} dBm</span>
                </div>
                <div class="info-item">
                    <span class="info-label">Timestamp:</span>
                    <span class="info-value">${formatPacketTime(packet.timestamp)}</span>
                </div>
                <div class="info-item">
                    <span class="info-label">PHY:</span>
                    <span class="info-value">${packet.phy || '-'}</span>
                </div>
                <div class="info-item">
                    <span class="info-label">Channel:</span>
                    <span class="info-value">${packet.channel || '-'}</span>
                </div>
                <div class="info-item">
                    <span class="info-label">Type:</span>
                    <span class="info-value">${packet.frame_type || '-'}</span>
                </div>
            </div>
        </div>

        <div class="packet-detail-section">
            <h4>🔍 Surowe dane (HEX)</h4>
            <div class="hex-raw">
                <code>${packet.advertising_data || 'Brak danych'}</code>
            </div>
        </div>

        <div class="packet-detail-section">
            <h4>🧩 Analiza bajt po bajcie</h4>
            <div class="hex-breakdown">
                ${parsed.structures.map(s => `
                    <div class="ad-structure">
                        <div class="ad-header">
                            <span class="ad-type-badge" style="background-color: ${getAdTypeColor(s.type)};">
                                0x${s.type.toString(16).padStart(2, '0').toUpperCase()}
                            </span>
                            <span class="ad-type-name">${s.typeName}</span>
                            <span class="ad-length">Length: ${s.length} bytes</span>
                        </div>
                        <div class="ad-bytes">
                            <span class="byte byte-length" title="Length">${s.lengthHex}</span>
                            <span class="byte byte-type" title="Type (${s.typeName})">${s.typeHex}</span>
                            ${s.dataBytes.map((b, i) => `
                                <span class="byte byte-data" title="Data[${i}]: 0x${b}">${b}</span>
                            `).join('')}
                        </div>
                        <div class="ad-description">
                            ${s.description}
                        </div>
                    </div>
                `).join('')}
            </div>
        </div>

        <div class="packet-detail-section">
            <h4>📊 Zrozumiałe dane</h4>
            <div class="parsed-data">
                ${parsed.summary.length > 0 ? parsed.summary.map(item => `
                    <div class="parsed-item">
                        <span class="parsed-label">${item.label}:</span>
                        <span class="parsed-value">${item.value}</span>
                    </div>
                `).join('') : '<p class="empty-msg">Brak sparsowanych danych</p>'}
            </div>
        </div>

        <div class="packet-detail-section">
            <h4>📖 Legenda typów AD</h4>
            <div class="ad-legend">
                <div class="legend-item">
                    <span class="legend-badge" style="background-color: #FFD700;">0x01</span>
                    <span>Flags</span>
                </div>
                <div class="legend-item">
                    <span class="legend-badge" style="background-color: #FF6B6B;">0xFF</span>
                    <span>Manufacturer Data</span>
                </div>
                <div class="legend-item">
                    <span class="legend-badge" style="background-color: #4ECDC4;">0x09</span>
                    <span>Complete Local Name</span>
                </div>
                <div class="legend-item">
                    <span class="legend-badge" style="background-color: #95E1D3;">0x08</span>
                    <span>Shortened Local Name</span>
                </div>
                <div class="legend-item">
                    <span class="legend-badge" style="background-color: #A8E6CF;">0x0A</span>
                    <span>TX Power Level</span>
                </div>
                <div class="legend-item">
                    <span class="legend-badge" style="background-color: #F38181;">0x16</span>
                    <span>Service Data 16-bit</span>
                </div>
            </div>
        </div>
    `;
    
    modal.classList.add('active');
}

function parseAdvertisingData(hexString) {
    if (!hexString || hexString === '-') {
        return { structures: [], summary: [] };
    }

    const result = {
        structures: [],
        summary: []
    };

    // Convert hex string to bytes
    const bytes = [];
    const cleaned = hexString.replace(/[^0-9A-Fa-f]/g, '');
    for (let i = 0; i < cleaned.length; i += 2) {
        if (i + 1 < cleaned.length) {
            bytes.push(parseInt(cleaned.substr(i, 2), 16));
        }
    }

    if (bytes.length === 0) return result;

    // Parse AD structures
    let pos = 0;
    while (pos < bytes.length) {
        const length = bytes[pos];
        if (length === 0 || pos + length + 1 > bytes.length) break;

        const type = bytes[pos + 1];
        const dataStart = pos + 2;
        const dataEnd = pos + 1 + length;
        const data = bytes.slice(dataStart, dataEnd);

        const structure = {
            lengthHex: bytes[pos].toString(16).padStart(2, '0').toUpperCase(),
            typeHex: type.toString(16).padStart(2, '0').toUpperCase(),
            length: length,
            type: type,
            typeName: getAdTypeName(type),
            dataBytes: data.map(b => b.toString(16).padStart(2, '0').toUpperCase()),
            description: parseAdType(type, data, result.summary)
        };

        result.structures.push(structure);
        pos += length + 1;
    }

    return result;
}

function getAdTypeName(type) {
    const names = {
        0x01: 'Flags',
        0x02: 'Incomplete List of 16-bit Service UUIDs',
        0x03: 'Complete List of 16-bit Service UUIDs',
        0x04: 'Incomplete List of 32-bit Service UUIDs',
        0x05: 'Complete List of 32-bit Service UUIDs',
        0x06: 'Incomplete List of 128-bit Service UUIDs',
        0x07: 'Complete List of 128-bit Service UUIDs',
        0x08: 'Shortened Local Name',
        0x09: 'Complete Local Name',
        0x0A: 'TX Power Level',
        0x14: 'List of 16-bit Service Solicitation UUIDs',
        0x15: 'List of 128-bit Service Solicitation UUIDs',
        0x16: 'Service Data - 16-bit UUID',
        0x19: 'Appearance',
        0x1A: 'Advertising Interval',
        0x20: 'Service Data - 32-bit UUID',
        0x21: 'Service Data - 128-bit UUID',
        0xFF: 'Manufacturer Specific Data'
    };
    return names[type] || `Unknown Type (0x${type.toString(16).padStart(2, '0')})`;
}

function parseAdType(type, data, summary) {
    let description = '';

    switch (type) {
        case 0x01: // Flags
            if (data.length > 0) {
                const flags = data[0];
                const flagNames = [];
                if (flags & 0x01) flagNames.push('LE Limited Discoverable');
                if (flags & 0x02) flagNames.push('LE General Discoverable');
                if (flags & 0x04) flagNames.push('BR/EDR Not Supported');
                if (flags & 0x08) flagNames.push('Simultaneous LE + BR/EDR (Controller)');
                if (flags & 0x10) flagNames.push('Simultaneous LE + BR/EDR (Host)');
                description = flagNames.length > 0 ? flagNames.join(', ') : 'No flags set';
                summary.push({ label: '🚩 Flags', value: description });
            }
            break;

        case 0x08: // Shortened Local Name
        case 0x09: // Complete Local Name
            try {
                const name = String.fromCharCode(...data);
                description = `Device Name: "${name}"`;
                summary.push({ label: '📛 Nazwa urządzenia', value: name });
            } catch {
                description = 'Invalid name data';
            }
            break;

        case 0x0A: // TX Power Level
            if (data.length > 0) {
                const power = data[0] > 127 ? data[0] - 256 : data[0]; // signed byte
                description = `TX Power: ${power} dBm`;
                summary.push({ label: '📡 TX Power', value: `${power} dBm` });
            }
            break;

        case 0xFF: // Manufacturer Specific Data
            if (data.length >= 2) {
                const companyId = data[0] | (data[1] << 8); // Little-endian
                const companyName = getCompanyName(companyId);
                const mfgData = data.slice(2).map(b => b.toString(16).padStart(2, '0')).join(' ').toUpperCase();
                description = `Company: ${companyName} (0x${companyId.toString(16).padStart(4, '0')})<br/>Data: ${mfgData || 'None'}`;
                summary.push({ label: '🏭 Producent', value: companyName });
                
                // Special parsing for known manufacturers
                if (companyId === 0x004C && data.length >= 4) {
                    // Apple iBeacon / AirTag etc
                    const appleType = data[2];
                    if (appleType === 0x02 && data.length >= 25) {
                        description += '<br/>🍎 <strong>Apple iBeacon</strong>';
                        summary.push({ label: '📱 Typ', value: 'Apple iBeacon' });
                    } else if (appleType === 0x12) {
                        description += '<br/>🍎 <strong>Find My (AirTag)</strong>';
                        summary.push({ label: '📱 Typ', value: 'Apple Find My' });
                    }
                } else if (companyId === 0x0006) {
                    description += '<br/>🪟 <strong>Microsoft Device</strong>';
                }
            }
            break;

        case 0x03: // Complete List of 16-bit Service UUIDs
        case 0x02: // Incomplete List of 16-bit Service UUIDs
            if (data.length >= 2) {
                const uuids = [];
                for (let i = 0; i < data.length; i += 2) {
                    if (i + 1 < data.length) {
                        const uuid = data[i] | (data[i + 1] << 8); // Little-endian
                        uuids.push(`0x${uuid.toString(16).padStart(4, '0')}`);
                    }
                }
                description = `Service UUIDs: ${uuids.join(', ')}`;
                summary.push({ label: '🔌 Serwisy', value: uuids.join(', ') });
            }
            break;

        case 0x16: // Service Data - 16-bit UUID
            if (data.length >= 2) {
                const serviceUuid = data[0] | (data[1] << 8);
                const serviceData = data.slice(2).map(b => b.toString(16).padStart(2, '0')).join(' ').toUpperCase();
                description = `Service UUID: 0x${serviceUuid.toString(16).padStart(4, '0')}<br/>Data: ${serviceData}`;
                summary.push({ label: '🔌 Service Data', value: `UUID: 0x${serviceUuid.toString(16).padStart(4, '0')}` });
            }
            break;

        case 0x19: // Appearance
            if (data.length >= 2) {
                const appearance = data[0] | (data[1] << 8);
                const appearanceName = getAppearanceName(appearance);
                description = `Appearance: ${appearanceName} (0x${appearance.toString(16).padStart(4, '0')})`;
                summary.push({ label: '👀 Wygląd', value: appearanceName });
            }
            break;

        default:
            description = `Raw data: ${data.map(b => b.toString(16).padStart(2, '0')).join(' ').toUpperCase()}`;
    }

    return description;
}

function getAdTypeColor(type) {
    const colors = {
        0x01: '#FFD700', // Flags - Gold
        0xFF: '#FF6B6B', // Manufacturer - Red
        0x09: '#4ECDC4', // Complete Name - Cyan
        0x08: '#95E1D3', // Shortened Name - Light cyan
        0x0A: '#A8E6CF', // TX Power - Green
        0x16: '#F38181', // Service Data - Pink
        0x03: '#AA96DA', // Services - Purple
        0x02: '#AA96DA', // Services - Purple
        0x19: '#FCBAD3'  // Appearance - Light pink
    };
    return colors[type] || '#95A5A6'; // Default gray
}

function getCompanyName(companyId) {
    const companies = {
        0x004C: 'Apple, Inc.',
        0x0006: 'Microsoft',
        0x00E0: 'Google',
        0x0075: 'Samsung Electronics Co. Ltd.',
        0x0059: 'Nordic Semiconductor ASA',
        0x0157: 'Samsung Electronics Co. Ltd.',
        0x01D5: 'Amazon.com Services LLC',
        0x038F: 'Shenzhen Jingxun Software',
        0x0268: 'Huawei Technologies Co., Ltd.',
        0x014D: 'Xiaomi Inc.',
        0x0089: 'Intel Corp.',
        0x025E: 'Broadcom',
        0x01A9: 'Realtek Semiconductor Corp.'
    };
    return companies[companyId] || `Unknown (0x${companyId.toString(16).padStart(4, '0')})`;
}

function getAppearanceName(appearance) {
    const appearances = {
        0: 'Unknown',
        64: 'Generic Phone',
        128: 'Generic Computer',
        192: 'Generic Watch',
        193: 'Watch: Sports Watch',
        256: 'Generic Clock',
        320: 'Generic Display',
        384: 'Generic Remote Control',
        448: 'Generic Eye-glasses',
        512: 'Generic Tag',
        576: 'Generic Keyring',
        640: 'Generic Media Player',
        704: 'Generic Barcode Scanner',
        768: 'Generic Thermometer',
        832: 'Generic Heart rate Sensor',
        833: 'Heart Rate Sensor: Heart Rate Belt',
        896: 'Generic Blood Pressure',
        960: 'Generic HID',
        961: 'Keyboard',
        962: 'Mouse',
        963: 'Joystick',
        964: 'Gamepad',
        1024: 'Generic Glucose Meter',
        1088: 'Generic Running Walking Sensor',
        1152: 'Generic Cycling',
        1216: 'Generic Pulse Oximeter',
        1280: 'Generic Weight Scale',
        1344: 'Generic Outdoor Sports Activity',
        3136: 'Generic Light Fixtures',
        3200: 'Generic Fan'
    };
    return appearances[appearance] || `Unknown (${appearance})`;
}

/* ==================== RSSI TREND TRACKING ==================== */

function switchTab(tabName) {
    // Hide all tabs
    document.querySelectorAll('.modal-tab-content').forEach(tab => {
        tab.classList.remove('active');
    });
    document.querySelectorAll('.modal-tab-btn').forEach(btn => {
        btn.classList.remove('active');
    });
    
    // Show selected tab
    const selectedTab = document.getElementById(tabName);
    if (selectedTab) {
        selectedTab.classList.add('active');
    }
    
    // Mark button as active
    event.target?.classList.add('active');
    
    // Load RSSI data if switching to rssi-tab
    if (tabName === 'rssi-tab' && !selectedTab.dataset.loaded) {
        const deviceMacs = document.querySelectorAll('[data-device-mac]');
        if (deviceMacs.length > 0) {
            const mac = deviceMacs[0].getAttribute('data-device-mac');
            loadRssiTrend(mac);
            selectedTab.dataset.loaded = 'true';
        }
    }
}

function loadRssiTrend(deviceMac) {
    const chartContainer = document.getElementById('rssi-chart');
    if (!chartContainer) return;
    
    // Show loading
    chartContainer.innerHTML = `
        <div class="loading-spinner">
            <div class="spinner"></div>
            <span>Loading RSSI trend data...</span>
        </div>
    `;
    
    fetch(`/api/devices/${encodeURIComponent(deviceMac)}/rssi-raw?limit=100`)
        .then(response => {
            if (!response.ok) {
                throw new Error(`HTTP ${response.status}`);
            }
            return response.json();
        })
        .then(data => {
            if (data.success && data.measurements && data.measurements.length > 0) {
                // Draw the trend chart
                drawRssiChart(data.measurements, chartContainer);
                
                // Update stats
                const avgRssi = parseFloat(data.rssi_stats.avg).toFixed(1);
                document.getElementById('rssi-avg').textContent = `${avgRssi} dBm`;
                document.getElementById('rssi-count').textContent = data.measurements_count;
                
                // Update trend indicator
                const trendDesc = data.trend.description;
                const trendDir = data.trend.direction;
                let trendEmoji = '➡️';
                if (trendDir === 'getting_closer') {
                    trendEmoji = '📶 Zbliża się';
                } else if (trendDir === 'getting_farther') {
                    trendEmoji = '📉 Oddala się';
                } else {
                    trendEmoji = '➡️ Stabilne';
                }
                document.getElementById('rssi-trend').textContent = trendEmoji;
                
                // Draw quality distribution bars
                drawQualityBars(data.signal_quality_distribution);
            } else {
                chartContainer.innerHTML = `
                    <div class="empty-msg" style="display: flex; align-items: center; justify-content: center; height: 100%;">
                        <span>No RSSI measurements available yet</span>
                    </div>
                `;
            }
        })
        .catch(error => {
            console.error('Error loading RSSI trend:', error);
            chartContainer.innerHTML = `
                <div class="empty-msg" style="display: flex; align-items: center; justify-content: center; height: 100%; color: var(--accent-red);">
                    <span>Error: ${error.message}</span>
                </div>
            `;
        });
}

function drawRssiChart(measurements, container) {
    if (!measurements || measurements.length === 0) {
        container.innerHTML = '<div class="empty-msg">No data</div>';
        return;
    }
    
    const width = container.clientWidth;
    const height = 400;
    const padding = 40;
    const chartWidth = width - padding * 2;
    const chartHeight = height - padding * 2;
    
    // Find min/max RSSI
    const rssiValues = measurements.map(m => m.rssi);
    const minRssi = Math.min(...rssiValues);
    const maxRssi = Math.max(...rssiValues);
    const rssiRange = maxRssi - minRssi || 10;
    
    // Create SVG
    const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    svg.setAttribute('width', '100%');
    svg.setAttribute('height', '100%');
    svg.setAttribute('viewBox', `0 0 ${width} ${height}`);
    svg.style.cssText = 'display: block;';
    
    // Draw grid lines
    for (let i = 0; i <= 5; i++) {
        const y = padding + (chartHeight / 5) * i;
        const rssiLevel = maxRssi - (rssiRange / 5) * i;
        
        const line = document.createElementNS('http://www.w3.org/2000/svg', 'line');
        line.setAttribute('x1', padding);
        line.setAttribute('y1', y);
        line.setAttribute('x2', width - padding);
        line.setAttribute('y2', y);
        line.setAttribute('stroke', '#1a2332');
        line.setAttribute('stroke-width', '0.5');
        svg.appendChild(line);
        
        // Add y-axis label
        const label = document.createElementNS('http://www.w3.org/2000/svg', 'text');
        label.setAttribute('x', padding - 10);
        label.setAttribute('y', y + 4);
        label.setAttribute('text-anchor', 'end');
        label.setAttribute('font-size', '12');
        label.setAttribute('fill', '#8b949e');
        label.textContent = `${Math.round(rssiLevel)}`;
        svg.appendChild(label);
    }
    
    // Draw data points and line
    let pathD = '';
    measurements.forEach((m, idx) => {
        const x = padding + (chartWidth / (measurements.length - 1 || 1)) * idx;
        const normalizedRssi = (m.rssi - minRssi) / rssiRange;
        const y = padding + chartHeight - (normalizedRssi * chartHeight);
        
        if (idx === 0) {
            pathD += `M ${x} ${y}`;
        } else {
            pathD += ` L ${x} ${y}`;
        }
    });
    
    // Draw line
    const path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    path.setAttribute('d', pathD);
    path.setAttribute('stroke', '#58a6ff');
    path.setAttribute('stroke-width', '2');
    path.setAttribute('fill', 'none');
    path.setAttribute('stroke-linecap', 'round');
    path.setAttribute('stroke-linejoin', 'round');
    svg.appendChild(path);
    
    // Draw points
    measurements.forEach((m, idx) => {
        const x = padding + (chartWidth / (measurements.length - 1 || 1)) * idx;
        const normalizedRssi = (m.rssi - minRssi) / rssiRange;
        const y = padding + chartHeight - (normalizedRssi * chartHeight);
        
        const color = m.signal_quality === 'excellent' ? '#3fb950' :
                     m.signal_quality === 'good' ? '#58a6ff' :
                     m.signal_quality === 'fair' ? '#d29922' :
                     m.signal_quality === 'poor' ? '#f85149' : '#d62828';
        
        const circle = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
        circle.setAttribute('cx', x);
        circle.setAttribute('cy', y);
        circle.setAttribute('r', '4');
        circle.setAttribute('fill', color);
        circle.setAttribute('stroke', '#0a0e27');
        circle.setAttribute('stroke-width', '2');
        circle.style.cursor = 'pointer';
        
        const title = document.createElementNS('http://www.w3.org/2000/svg', 'title');
        title.textContent = `${m.rssi} dBm (${m.signal_quality}) @ ${new Date(m.timestamp).toLocaleTimeString()}`;
        circle.appendChild(title);
        
        svg.appendChild(circle);
    });
    
    // Draw axes
    const xAxis = document.createElementNS('http://www.w3.org/2000/svg', 'line');
    xAxis.setAttribute('x1', padding);
    xAxis.setAttribute('y1', padding + chartHeight);
    xAxis.setAttribute('x2', width - padding);
    xAxis.setAttribute('y2', padding + chartHeight);
    xAxis.setAttribute('stroke', '#3a4a5e');
    xAxis.setAttribute('stroke-width', '1');
    svg.appendChild(xAxis);
    
    const yAxis = document.createElementNS('http://www.w3.org/2000/svg', 'line');
    yAxis.setAttribute('x1', padding);
    yAxis.setAttribute('y1', padding);
    yAxis.setAttribute('x2', padding);
    yAxis.setAttribute('y2', padding + chartHeight);
    yAxis.setAttribute('stroke', '#3a4a5e');
    yAxis.setAttribute('stroke-width', '1');
    svg.appendChild(yAxis);
    
    // Draw time labels on X axis
    const timeLabels = 5;
    for (let i = 0; i <= timeLabels; i++) {
        const idx = Math.floor((measurements.length - 1) * i / timeLabels);
        const m = measurements[idx];
        if (m && m.timestamp) {
            const x = padding + (chartWidth / timeLabels) * i;
            const timeLabel = document.createElementNS('http://www.w3.org/2000/svg', 'text');
            timeLabel.setAttribute('x', x);
            timeLabel.setAttribute('y', height - 10);
            timeLabel.setAttribute('text-anchor', 'middle');
            timeLabel.setAttribute('font-size', '10');
            timeLabel.setAttribute('fill', '#8b949e');
            timeLabel.textContent = new Date(m.timestamp).toLocaleTimeString([], {hour: '2-digit', minute: '2-digit', second: '2-digit'});
            svg.appendChild(timeLabel);
        }
    }
    
    // Clear and add SVG
    container.innerHTML = '';
    container.appendChild(svg);
}

function drawQualityBars(distribution) {
    const container = document.getElementById('quality-bars');
    if (!container) return;
    
    const total = Object.values(distribution).reduce((a, b) => a + b, 0) || 1;
    const qualities = ['excellent', 'good', 'fair', 'poor', 'very_poor'];
    const labels = ['Excellent', 'Good', 'Fair', 'Poor', 'Very Poor'];
    
    container.innerHTML = qualities.map((q, i) => {
        const count = distribution[q] || 0;
        const percent = ((count / total) * 100).toFixed(0);
        return `
            <div style="flex: 1; display: flex; flex-direction: column; align-items: center; gap: 4px;">
                <div class="quality-bar ${q}" style="height: ${Math.max(5, percent)}%; width: 100%;"></div>
                <span style="font-size: 0.7rem; color: var(--text-secondary);">${count}</span>
            </div>
        `;
    }).join('');
}
