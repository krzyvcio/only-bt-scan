// ═══════════════════════════════════════════════════════════════════════════
// MAIN APP - Główna logika aplikacji
// ═══════════════════════════════════════════════════════════════════════════

let pollTimers = {};

document.addEventListener('DOMContentLoaded', () => {
    initApp();
});

function initApp() {
    moment.locale('pl');
    setupEventListeners();
    setupModals();
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

    // Tab switching
    const tabButtons = document.querySelectorAll('.tab-button');
    tabButtons.forEach(btn => {
        btn.addEventListener('click', () => {
            const tabId = btn.dataset.tab;
            
            // Update active tab button
            tabButtons.forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            
            // Show corresponding tab content
            document.querySelectorAll('.tab-content').forEach(content => {
                content.classList.remove('active');
            });
            
            const targetContent = document.getElementById(`tab-${tabId}`);
            if (targetContent) {
                targetContent.classList.add('active');
            }
            
            // Load data for specific tabs
            if (tabId === 'telemetry') {
                loadTelemetry();
            } else if (tabId === 'rssi') {
                const firstDevice = document.querySelector('#devices-tbody tr[data-mac]');
                if (firstDevice) {
                    const mac = firstDevice.dataset.mac;
                    loadRssiTrend(mac);
                }
            } else if (tabId === 'correlations') {
                loadCorrelations();
            } else if (tabId === 'anomalies') {
                loadAnomalies();
            }
        });
    });

    // Auto-scroll toggle
    const autoScrollBtn = document.getElementById('auto-scroll-toggle');
    if (autoScrollBtn) {
        autoScrollBtn.addEventListener('click', toggleAutoScroll);
    }

    // Search functionality
    const searchInput = document.getElementById('search-input');
    if (searchInput) {
        searchInput.addEventListener('input', (e) => {
            filterDevices();
        });
    }

    // Filter dropdowns
    const filterRssi = document.getElementById('filter-rssi');
    if (filterRssi) {
        filterRssi.addEventListener('change', filterDevices);
    }
    const filterManufacturer = document.getElementById('filter-manufacturer');
    if (filterManufacturer) {
        filterManufacturer.addEventListener('change', filterDevices);
    }
    const filterMacType = document.getElementById('filter-mac-type');
    if (filterMacType) {
        filterMacType.addEventListener('change', filterDevices);
    }
}

function loadData() {
    loadDevices();
    loadPackets();
    loadStats();
}

function startPolling() {
    // Devices polling (every 2 seconds)
    pollTimers.devices = setInterval(() => {
        loadDevices();
    }, POLL_INTERVAL);

    // Packets polling (every 1 second)
    pollTimers.packets = setInterval(() => {
        loadPackets();
    }, PACKET_POLL_INTERVAL);
}

function stopPolling() {
    Object.values(pollTimers).forEach(timer => clearInterval(timer));
    pollTimers = {};
}

async function loadStats() {
    const data = await loadStatsFromApi();
    if (data) {
        document.getElementById('total-devices').textContent = data.total_devices || 0;
        document.getElementById('total-packets').textContent = data.total_packets || 0;
        document.getElementById('scan-sessions').textContent = data.scan_sessions || 0;
    }
}

async function loadTelemetry() {
    const data = await loadTelemetryFromApi();
    // Render telemetry data...
}

async function loadCorrelations() {
    const data = await loadTemporalCorrelations();
    // Render correlations...
}

async function loadAnomalies() {
    // Load anomalies for first device or selected...
}

function filterDevices() {
    const query = document.getElementById('search-input')?.value?.toLowerCase() || '';
    const rssiFilter = document.getElementById('filter-rssi')?.value || '';
    const manufacturerFilter = document.getElementById('filter-manufacturer')?.value || '';
    const macTypeFilter = document.getElementById('filter-mac-type')?.value || '';
    
    const rows = document.querySelectorAll('#devices-tbody tr');
    
    rows.forEach(row => {
        const text = row.textContent.toLowerCase();
        const rssiValue = row.querySelector('.rssi-value');
        const manufacturerCell = row.querySelector('.manufacturer');
        const macTypeCell = row.querySelector('.mac-type');
        
        const matchesText = query === '' || text.includes(query);
        
        let matchesRssi = true;
        if (rssiFilter) {
            const rssi = rssiValue ? parseInt(rssiValue.textContent) : -100;
            switch (rssiFilter) {
                case 'excellent': matchesRssi = rssi >= -50; break;
                case 'good': matchesRssi = rssi >= -60 && rssi < -50; break;
                case 'fair': matchesRssi = rssi >= -70 && rssi < -60; break;
                case 'poor': matchesRssi = rssi < -70; break;
            }
        }
        
        const matchesManufacturer = !manufacturerFilter || 
            (manufacturerCell && manufacturerCell.textContent.toLowerCase() === manufacturerFilter.toLowerCase());
        
        const matchesMacType = !macTypeFilter || 
            (macTypeCell && macTypeCell.textContent.toLowerCase().includes(macTypeFilter.toLowerCase()));
        
        row.style.display = (matchesText && matchesRssi && matchesManufacturer && matchesMacType) ? '' : 'none';
    });
}

function populateManufacturerFilter() {
    const manufacturers = new Set();
    const rows = document.querySelectorAll('#devices-tbody tr[data-mac]');
    rows.forEach(row => {
        const mfgCell = row.querySelector('.manufacturer');
        if (mfgCell && mfgCell.textContent && mfgCell.textContent !== '-') {
            manufacturers.add(mfgCell.textContent.trim());
        }
    });
    
    const select = document.getElementById('filter-manufacturer');
    if (select) {
        const currentValue = select.value;
        select.innerHTML = '<option value="">Producent (wszystkie)</option>';
        Array.from(manufacturers).sort().forEach(mfg => {
            const option = document.createElement('option');
            option.value = mfg;
            option.textContent = mfg;
            select.appendChild(option);
        });
        select.value = currentValue;
    }
}

function formatTimestamp(ts) {
    if (!ts) return '-';
    const date = new Date(ts);
    return date.toLocaleString('pl-PL');
}

function showToast(type, message) {
    const toast = document.createElement('div');
    toast.className = `toast toast-${type}`;
    toast.textContent = message;
    document.body.appendChild(toast);
    
    setTimeout(() => {
        toast.classList.add('show');
    }, 10);
    
    setTimeout(() => {
        toast.classList.remove('show');
        setTimeout(() => toast.remove(), 300);
    }, 3000);
}

// Advertising Data Parser
function parseAdvertisingData(hexString) {
    if (!hexString) return { structures: [], summary: [] };
    
    const bytes = hexString.split(/\s+/).map(b => parseInt(b, 16));
    const structures = [];
    let i = 0;
    
    while (i < bytes.length) {
        const length = bytes[i];
        if (length === 0 || i + length > bytes.length) break;
        
        const type = bytes[i + 1];
        const data = bytes.slice(i + 2, i + 1 + length);
        
        const typeInfo = getAdTypeInfo(type);
        
        structures.push({
            length,
            type,
            typeHex: type.toString(16).padStart(2, '0').toUpperCase(),
            lengthHex: length.toString(16).padStart(2, '0').toUpperCase(),
            typeName: typeInfo.name,
            dataBytes: data,
            description: typeInfo.description,
            isStructure: true
        });
        
        i += 1 + length;
    }
    
    const summary = [];
    structures.forEach(s => {
        if (s.type === 0x09) { // Complete Local Name
            const name = String.fromCharCode(...s.dataBytes);
            summary.push({ label: 'Nazwa urządzenia', value: name });
        } else if (s.type === 0xFF) { // Manufacturer
            if (s.dataBytes.length >= 2) {
                const mfg = (s.dataBytes[1] << 8) | s.dataBytes[0];
                summary.push({ label: 'ID Producenta', value: `0x${mfg.toString(16).toUpperCase()}` });
            }
        } else if (s.type === 0x0A) { // TX Power
            if (s.dataBytes.length > 0) {
                summary.push({ label: 'Moc nadawania', value: `${s.dataBytes[0]} dBm` });
            }
        }
    });
    
    return { structures, summary };
}

function getAdTypeInfo(type) {
    const types = {
        0x01: { name: 'Flags', description: 'Flagi urządzenia' },
        0x02: { name: 'Incomplete List 16-bit UUIDs', description: 'Niekompletna lista UUID 16-bit' },
        0x03: { name: 'Complete List 16-bit UUIDs', description: 'Kompletna lista UUID 16-bit' },
        0x04: { name: 'Incomplete List 32-bit UUIDs', description: 'Niekompletna lista UUID 32-bit' },
        0x05: { name: 'Complete List 32-bit UUIDs', description: 'Kompletna lista UUID 32-bit' },
        0x06: { name: 'Incomplete List 128-bit UUIDs', description: 'Niekompletna lista UUID 128-bit' },
        0x07: { name: 'Complete List 128-bit UUIDs', description: 'Kompletna lista UUID 128-bit' },
        0x08: { name: 'Shortened Local Name', description: 'Skrócona nazwa lokalna' },
        0x09: { name: 'Complete Local Name', description: 'Pełna nazwa lokalna' },
        0x0A: { name: 'TX Power Level', description: 'Poziom mocy nadawania' },
        0x0D: { name: 'Device Name', description: 'Nazwa urządzenia' },
        0x14: { name: 'Slave Connection Interval', description: 'Interwał połączenia slave' },
        0x1F: { name: 'List of 16-bit Service UUIDs', description: 'Lista serwisów 16-bit' },
        0x20: { name: 'List of 32-bit Service UUIDs', description: 'Lista serwisów 32-bit' },
        0x21: { name: 'List of 128-bit Service UUIDs', description: 'Lista serwisów 128-bit' },
        0x22: { name: 'Public Target Address', description: 'Publiczny adres docelowy' },
        0x23: { name: 'Random Target Address', description: 'Losowy adres docelowy' },
        0x24: { name: 'Appearance', description: 'Wygląd urządzenia' },
        0x27: { name: 'Advertising Interval', description: 'Interwał reklamy' },
        0x29: { name: 'Manufacturer Data', description: 'Dane producenta' },
        0x2A: { name: 'Service Data', description: 'Dane serwisu' },
        0x2A: { name: 'Service Data 16-bit UUID', description: 'Dane serwisu 16-bit UUID' },
        0x2A19: { name: 'Battery Level', description: 'Poziom baterii' },
        0xFF: { name: 'Manufacturer Specific Data', description: 'Dane specyficzne producenta' },
    };
    
    return types[type] || { name: `Unknown (0x${type.toString(16)})`, description: 'Nieznany typ' };
}

function getAdTypeColor(type) {
    const colors = {
        0x01: '#4ade80',
        0x09: '#60a5fa',
        0x0A: '#fbbf24',
        0xFF: '#f87171',
        0x16: '#c084fc',
        0x02: '#22d3ee',
        0x03: '#22d3ee',
    };
    return colors[type] || '#6b7280';
}

// Pagination functions (call from other modules)
function updatePagination(type) {
    if (type === 'devices') updateDevicesPagination();
    else if (type === 'packets') updatePacketsPagination();
}
