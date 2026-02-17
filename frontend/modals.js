// ═══════════════════════════════════════════════════════════════════════════
// MODALS - Obsługa okien modalowych
// ═══════════════════════════════════════════════════════════════════════════

// PACKET DETAILS MODAL
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
                    <button class="btn-rssi-track" onclick="trackRssi('${packet.mac_address}')" title="Śledź RSSI tego urządzenia">
                        📶 RSSI
                    </button>
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
                            ${s.dataBytes.map((b, i) => `<span class="byte byte-data" title="Data[${i}]: 0x${b}">${b}</span>`).join('')}
                        </div>
                        <div class="ad-description">${s.description}</div>
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
    `;

    modal.classList.add('active');
}

// DEVICE HISTORY MODAL
async function showDeviceHistory(mac) {
    const modal = document.getElementById('device-history-modal');
    const modalBody = document.getElementById('modal-body'); // Matching index.html
    const deviceNameHeader = document.getElementById('modal-device-name');

    if (deviceNameHeader) deviceNameHeader.textContent = `Historia urządzenia: ${mac}`;
    if (modalBody) modalBody.innerHTML = '<div class="loading-spinner"><div class="spinner"></div><span>Ładowanie informacji...</span></div>';

    modal.classList.add('active');

    // Default to info tab
    switchTab('info-tab');

    const data = await loadDeviceHistory(mac);

    if (data && data.scan_history && data.scan_history.length > 0) {
        modalBody.innerHTML = `
            <table class="history-table">
                <thead>
                    <tr>
                        <th>Czas</th>
                        <th>RSSI</th>
                        <th>Akcja</th>
                    </tr>
                </thead>
                <tbody>
                    ${data.scan_history.map(h => `
                        <tr>
                            <td>${formatTimestamp(h.scan_timestamp)}</td>
                            <td class="${getRssiClass(h.rssi)}">${h.rssi} dBm</td>
                            <td>Skan #${h.scan_number}</td>
                        </tr>
                    `).join('')}
                </tbody>
            </table>
        `;
    } else {
        modalBody.innerHTML = `<p class="empty-msg">Brak zapisanych danych historycznych dla ${mac}</p>`;
    }

    // Load RSSI chart if tab is switched
    if (typeof loadRssiTrend === 'function') {
        loadRssiTrend(mac);
    }
}

function switchTab(tabId) {
    // Hide all tab contents
    document.querySelectorAll('.modal-tab-content').forEach(tab => {
        tab.classList.remove('active');
    });

    // Deactivate all tab buttons
    document.querySelectorAll('.modal-tab-btn').forEach(btn => {
        btn.classList.remove('active');
    });

    // Show selected tab content
    const selectedTab = document.getElementById(tabId);
    if (selectedTab) {
        selectedTab.classList.add('active');
    }

    // Activate clicked button
    const btn = document.querySelector(`.modal-tab-btn[onclick="switchTab('${tabId}')"]`);
    if (btn) {
        btn.classList.add('active');
    }
}

function trackRssi(mac) {
    // Switch to RSSI tab and initialize tracking
    showDeviceHistory(mac);
    setTimeout(() => switchTab('rssi-tab'), 100);
}

function closeModal(modalId) {
    const modal = document.getElementById(modalId);
    if (modal) modal.classList.remove('active');
}

// Setup modal close handlers
function setupModals() {
    // Packet details modal
    const closePacketModal = document.getElementById('close-packet-modal');
    if (closePacketModal) {
        closePacketModal.addEventListener('click', () => {
            closeModal('packet-details-modal');
        });
    }

    // Device history modal
    const closeHistoryModal = document.getElementById('close-modal'); // Corrected to match index.html
    if (closeHistoryModal) {
        closeHistoryModal.addEventListener('click', () => {
            closeModal('device-history-modal');
        });
    }

    // Close on background click
    document.querySelectorAll('.modal').forEach(modal => {
        modal.addEventListener('click', (e) => {
            if (e.target === modal) {
                modal.classList.remove('active');
            }
        });
    });
}
