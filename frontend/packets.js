// ═══════════════════════════════════════════════════════════════════════════
// PACKETS - Obsługa surowych pakietów
// ═══════════════════════════════════════════════════════════════════════════

let packets = [];
let lastPacketId = 0;
let packetsPage = 1;
let packetsTotalPages = 1;
let packetsTotal = 0;
let isAutoScrollEnabled = true;

function formatPacketTime(timestamp) {
    if (!timestamp) return '-';
    const date = new Date(timestamp);
    return date.toLocaleTimeString('pl-PL', {
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
        fractionalDigitDigits: 3
    });
}

function renderPackets(packetList) {
    const list = document.getElementById('packets-list');

    if (!packetList || packetList.length === 0) {
        list.innerHTML = `
            <div class="empty-packets">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M22 12h-4l-3 9L9 3l-3 9H2"/>
                </svg>
                <span>Oczekiwanie na pakiety...</span>
            </div>
        `;
        return;
    }

    list.innerHTML = packetList.map((packet, index) => {
        const timeStr = formatPacketTime(packet.timestamp);
        const isNew = index === 0 && packet.id > lastPacketId;
        const rssiClass = typeof getRssiClass === 'function' ? getRssiClass(packet.rssi) : '';

        return `
            <div class="packet-row ${isNew ? 'new' : ''}" 
                onclick="showPacketDetails(${index})" 
                style="cursor: pointer;"
                title="Kliknij aby zobaczyć szczegóły">
                <span class="packet-time">${timeStr}</span>
                <span class="packet-mac">${packet.mac_address}</span>
                <span class="packet-rssi ${rssiClass}">${packet.rssi} dBm</span>
                <span class="packet-data">${(packet.advertising_data || '').substring(0, 40)}...</span>
            </div>
        `;
    }).join('');

    // Auto scroll for new packets
    if (isAutoScrollEnabled && packetList.length > 0) {
        if (list) {
            list.scrollTop = 0;
        }
    }
}

async function loadPackets() {
    const data = await loadPacketsFromApi();
    if (data && data.data) {
        // Update lastPacketId tracking
        if (data.data.length > 0) {
            const maxId = Math.max(...data.data.map(p => p.id || 0));
            if (maxId > lastPacketId) {
                lastPacketId = maxId;
            }
        }

        packets = data.data;
        packetsTotal = data.total || packets.length;
        packetsTotalPages = Math.ceil(packetsTotal / 100);
        packetsPage = data.page || 1;

        renderPackets(packets);
        updatePacketsPagination();
    }
}

function updatePacketsPagination() {
    const pageInfo = document.getElementById('packets-page-info');
    if (pageInfo) {
        pageInfo.textContent = `Strona ${packetsPage} / ${packetsTotalPages} (${packetsTotal} pakietów)`;
    }
}

// Initialize from checkbox
document.addEventListener('DOMContentLoaded', () => {
    const autoScrollBtn = document.getElementById('auto-scroll');
    if (autoScrollBtn) {
        isAutoScrollEnabled = autoScrollBtn.checked;
    }
});
