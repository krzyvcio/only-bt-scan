// ═══════════════════════════════════════════════════════════════════════════
// DEVICES - Obsługa urządzeń
// ═══════════════════════════════════════════════════════════════════════════

let devices = [];
let devicesPage = 1;
let devicesTotalPages = 1;
let devicesTotal = 0;

function getSignalClass(rssi) {
    if (rssi >= -50) return { bars: 4, class: 'excellent', label: 'Excellent' };
    if (rssi >= -60) return { bars: 3, class: 'good', label: 'Good' };
    if (rssi >= -70) return { bars: 2, class: 'fair', label: 'Fair' };
    if (rssi >= -80) return { bars: 1, class: 'poor', label: 'Poor' };
    return { bars: 0, class: 'very-poor', label: 'Very Poor' };
}

function getSignalBars(rssi) {
    const signal = getSignalClass(rssi);
    let html = '<div class="signal-bar-container">';
    for (let i = 0; i < 4; i++) {
        const active = i < signal.bars ? 'active' : '';
        html += `<div class="signal-bar ${active}"></div>`;
    }
    html += '</div>';
    return html;
}

function getRssiClass(rssi) {
    if (rssi >= -50) return 'rssi-excellent';
    if (rssi >= -60) return 'rssi-good';
    if (rssi >= -70) return 'rssi-fair';
    if (rssi >= -80) return 'rssi-poor';
    return 'rssi-very-poor';
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

        // Prepare MAC type display
        let macTypeCode = device.mac_type || '-';
        let macTypeClass = '';

        if (macTypeCode.includes('Public')) {
            macTypeClass = 'mac-type-public';
        } else if (macTypeCode.includes('Static')) {
            macTypeClass = 'mac-type-static';
        } else if (macTypeCode.includes('RPA') || device.is_rpa) {
            macTypeClass = 'mac-type-rpa';
            if (macTypeCode === '-') macTypeCode = 'RPA (Resolvable)';
        } else if (macTypeCode.includes('Non-Resolvable')) {
            macTypeClass = 'mac-type-nrpa';
        }

        return `
            <tr data-mac="${device.mac_address}">
                <td>
                    <div class="signal-bars">${getSignalBars(device.rssi)}</div>
                    <span class="rssi-value ${rssiClass}">${device.rssi} dBm</span>
                </td>
                <td>
                    <span class="device-name" title="${device.device_name || 'Brak nazwy'}">
                        ${device.device_name || '<em>Brak nazwy</em>'}
                    </span>
                </td>
                <td><span class="mac-address">${device.mac_address}</span></td>
                <td><span class="manufacturer" title="${manufacturer}">${manufacturer}</span></td>
                <td>
                    <span class="mac-type-badge ${macTypeClass} mac-type">
                        ${macTypeCode}
                    </span>
                </td>
                <td>
                    <span class="security-badge ${device.security_level === 'Secure Connections' ? 'secure' : ''}">${securityLevel}</span>
                </td>
                <td><span class="detection-count">${device.number_of_scan || 1}</span></td>
                <td><span class="timestamp" title="${firstSeen}">${firstSeen}</span></td>
                <td><span class="timestamp" title="${lastSeen}">${lastSeen}</span></td>
                <td>
                    <button class="history-btn" onclick="showDeviceHistory('${device.mac_address}')">Wyświetl</button>
                </td>
            </tr>
        `;
    }).join('');
}

async function loadDevices() {
    const data = await loadDevicesFromApi();
    if (data && data.data) {
        devices = data.data;
        devicesTotal = data.total || devices.length;
        devicesTotalPages = Math.ceil(devicesTotal / 50);
        devicesPage = data.page || 1;
        renderDevices(devices);
        if (typeof populateManufacturerFilter === 'function') {
            populateManufacturerFilter();
        }
        if (typeof updateDevicesPagination === 'function') {
            updateDevicesPagination();
        }
    }
}

function updateDevicesPagination() {
    const pageInfo = document.getElementById('devices-page-info');
    if (pageInfo) {
        pageInfo.textContent = `Strona ${devicesPage} / ${devicesTotalPages} (${devicesTotal} urządzeń)`;
    }
}
