const API_BASE = '/api';
const POLL_INTERVAL = 2000;
const PACKET_POLL_INTERVAL = 1000;

let devices = [];
let packets = [];
let lastPacketId = 0;
let isAutoScrollEnabled = true;
let pollTimers = {};

document.addEventListener('DOMContentLoaded', () => {
    initApp();
});

function initApp() {
    setupEventListeners();
    loadData();
    startPolling();
    showToast('info', 'Connected to scanner panel');
}

function setupEventListeners() {
    const refreshBtn = document.getElementById('refresh-btn');
    if (refreshBtn) {
        refreshBtn.addEventListener('click', () => {
            loadData();
            showToast('info', 'Refreshing data...');
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
}

function startPolling() {
    pollTimers.devices = setInterval(loadDevices, POLL_INTERVAL);
    pollTimers.packets = setInterval(loadPackets, PACKET_POLL_INTERVAL);
    pollTimers.stats = setInterval(loadStats, POLL_INTERVAL);
}

function stopPolling() {
    Object.values(pollTimers).forEach(timer => clearInterval(timer));
    pollTimers = {};
}

async function loadData() {
    await Promise.all([loadDevices(), loadPackets(), loadStats()]);
}

async function loadDevices() {
    try {
        const response = await fetch(`${API_BASE}/devices`);
        if (!response.ok) throw new Error('Failed to fetch devices');
        
        devices = await response.json();
        renderDevices(devices);
    } catch (error) {
        console.error('Error loading devices:', error);
    }
}

async function loadPackets() {
    try {
        const response = await fetch(`${API_BASE}/raw-packets`);
        if (!response.ok) throw new Error('Failed to fetch packets');
        
        const newPackets = await response.json();
        
        if (newPackets.length > 0 && newPackets[0].id !== lastPacketId) {
            const oldLength = packets.length;
            packets = newPackets.slice(0, 100);
            
            if (packets.length > oldLength || packets[0]?.id !== lastPacketId) {
                renderPackets();
                lastPacketId = packets[0]?.id || 0;
            }
        }
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
                <td colspan="7">
                    <div class="empty-state">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <rect x="2" y="7" width="20" height="14" rx="2" ry="2"/>
                            <path d="M16 21V5a2 2 0 00-2-2h-4a2 2 0 00-2 2v16"/>
                        </svg>
                        <span>No devices found. Scanning...</span>
                    </div>
                </td>
            </tr>
        `;
        return;
    }

    tbody.innerHTML = deviceList.map(device => {
        const signalClass = getSignalClass(device.rssi);
        const rssiClass = getRssiClass(device.rssi);
        const firstSeen = device.first_seen ? formatTimestamp(device.first_seen) : '-';
        const lastSeen = device.last_seen ? formatTimestamp(device.last_seen) : '-';
        
        return `
            <tr data-mac="${device.mac_address}">
                <td>
                    <div class="signal-bars">
                        ${getSignalBars(device.rssi)}
                    </div>
                    <span class="rssi-value ${rssiClass}">${device.rssi} dBm</span>
                </td>
                <td>
                    <span class="device-name" title="${device.device_name || 'Unknown'}">
                        ${device.device_name || '<em>Unknown</em>'}
                    </span>
                </td>
                <td>
                    <span class="mac-address">${device.mac_address}</span>
                </td>
                <td>
                    <span class="manufacturer" title="${device.manufacturer_name || 'Unknown'}">
                        ${device.manufacturer_name || '-'}
                    </span>
                </td>
                <td>
                    <span class="detection-count">${device.number_of_scan || 1}</span>
                </td>
                <td>
                    <span class="last-seen">${lastSeen}</span>
                </td>
                <td>
                    <button class="history-btn" onclick="showDeviceHistory('${device.mac_address}')">View</button>
                </td>
            </tr>
        `;
    }).join('');
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
        <div class="packet-row ${index === 0 ? 'new' : ''}">
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
        const date = new Date(timestamp);
        const now = new Date();
        const diff = now - date;
        
        if (diff < 60000) return 'Just now';
        if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
        if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`;
        
        return date.toLocaleDateString('pl-PL', { 
            month: 'short', 
            day: 'numeric',
            hour: '2-digit', 
            minute: '2-digit'
        });
    } catch {
        return '-';
    }
}

function formatPacketTime(timestamp) {
    if (!timestamp) return '-';
    try {
        const date = new Date(timestamp);
        return date.toLocaleTimeString('pl-PL', { 
            hour: '2-digit', 
            minute: '2-digit', 
            second: '2-digit',
            hour12: false
        });
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
        
        const device = data.device;
        const scanHistory = data.scan_history || [];
        const packets = data.packet_history || [];
        
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
