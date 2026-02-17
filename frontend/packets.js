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
    const tbody = document.getElementById('packets-tbody');
    
    if (!packetList || packetList.length === 0) {
        tbody.innerHTML = `
            <tr>
                <td colspan="7">
                    <div class="empty-state">
                        <span>Brak przechwyconych pakietów...</span>
                    </div>
                </td>
            </tr>
        `;
        return;
    }

    tbody.innerHTML = packetList.map((packet, index) => {
        const timeStr = formatPacketTime(packet.timestamp);
        const isNew = index === 0 && packet.id > lastPacketId;
        
        return `
            <tr class="packet-row ${isNew ? 'new' : ''}" 
                onclick="showPacketDetails(${index})" 
                style="cursor: pointer;"
                title="Kliknij aby zobaczyć szczegóły">
                <td><span class="packet-time">${timeStr}</span></td>
                <td><span class="packet-mac">${packet.mac_address}</span></td>
                <td><span class="packet-rssi ${getRssiClass(packet.rssi)}">${packet.rssi} dBm</span></td>
                <td><span class="packet-channel">${packet.channel || '-'}</span></td>
                <td><span class="packet-phy">${packet.phy || '-'}</span></td>
                <td><span class="packet-type">${packet.frame_type || '-'}</span></td>
                <td><span class="packet-data">${(packet.advertising_data || '').substring(0, 20)}...</span></td>
            </tr>
        `;
    }).join('');

    // Auto scroll to top for new packets
    if (isAutoScrollEnabled && packetList.length > 0) {
        const container = document.getElementById('packets-table-container');
        if (container) {
            container.scrollTop = 0;
        }
    }
}

async function loadPackets() {
    const data = await loadPacketsFromApi();
    if (data && data.packets) {
        // Update lastPacketId tracking
        if (data.packets.length > 0) {
            const maxId = Math.max(...data.packets.map(p => p.id || 0));
            if (maxId > lastPacketId) {
                lastPacketId = maxId;
            }
        }
        
        packets = data.packets;
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

function toggleAutoScroll() {
    isAutoScrollEnabled = !isAutoScrollEnabled;
    const btn = document.getElementById('auto-scroll-toggle');
    if (btn) {
        btn.textContent = isAutoScrollEnabled ? '🔄 Auto' : '⏸️ Manual';
        btn.classList.toggle('active', isAutoScrollEnabled);
    }
}
