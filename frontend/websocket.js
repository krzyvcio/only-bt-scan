// ═══════════════════════════════════════════════════════════════════════════
// WEBSOCKET - Komunikacja w czasie rzeczywistym
// ═══════════════════════════════════════════════════════════════════════════

let socket = null;
let isWsConnected = false;
let reconnectInterval = 5000;

function initWebSocket() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.host}/ws`;

    console.log('🔌 Inicjalizacja połączenia WebSocket...', wsUrl);

    try {
        socket = new WebSocket(wsUrl);

        socket.onopen = () => {
            isWsConnected = true;
            console.log('✅ WebSocket połączony');
            showToast('success', 'Połączono w czasie rzeczywistym');

            // Subskrypcja kanałów
            const channels = ['devices', 'raw_packets', 'telemetry'];
            channels.forEach(channel => {
                socket.send(JSON.stringify({ action: 'subscribe', channel }));
            });

            // Zatrzymujemy polling jeśli WS działa
            if (typeof stopPolling === 'function') {
                stopPolling();
                console.log('ℹ️ Wyłączono polling (używanie WebSocket)');
            }
        };

        socket.onmessage = (event) => {
            try {
                const msg = JSON.parse(event.data);
                handleWsMessage(msg);
            } catch (e) {
                console.error('❌ Błąd parsowania wiadomości WS:', e);
            }
        };

        socket.onclose = (event) => {
            isWsConnected = false;
            console.log(`❌ WebSocket rozłączony (kod: ${event.code})`);

            // Wracamy do pollingu w razie awarii WS
            if (typeof startPolling === 'function') {
                startPolling();
                console.log('ℹ️ Przywrócono polling (WebSocket rozłączony)');
            }

            // Próba ponownego połączenia
            setTimeout(initWebSocket, reconnectInterval);
        };

        socket.onerror = (error) => {
            console.error('⚠️ Błąd WebSocket:', error);
        };
    } catch (e) {
        console.error('❌ Nie udało się utworzyć WebSocket:', e);
    }
}

function handleWsMessage(msg) {
    if (!msg || !msg.channel) return;

    switch (msg.channel) {
        case 'devices':
            if (msg.data) {
                updateOrAddDevice(msg.data);
            }
            break;
        case 'raw_packets':
            if (msg.data) {
                addRealtimePacket(msg.data);
            }
            break;
        case 'telemetry':
            if (msg.data && typeof updateTelemetryUI === 'function') {
                updateTelemetryUI(msg.data);
            }
            break;
        default:
            console.debug('Wiadomość WS z nieznanego kanału:', msg.channel);
    }
}

function updateOrAddDevice(device) {
    if (!device || !device.mac_address) return;

    // Pobierz globalną tablicę urządzeń z devices.js (jeśli dostępna)
    if (typeof devices === 'undefined') return;

    const index = devices.findIndex(d => d.mac_address === device.mac_address);
    if (index !== -1) {
        // Aktualizacja istniejącego
        devices[index] = { ...devices[index], ...device };
    } else {
        // Dodanie nowego na początku
        devices.unshift(device);
        // Limit do 50 na pierwszej stronie (uproszczenie)
        if (devices.length > 50) devices.pop();
    }

    // Odśwież widok
    if (typeof renderDevices === 'function') {
        renderDevices(devices);
    }
}

function addRealtimePacket(packet) {
    if (!packet) return;

    // Pobierz globalną tablicę pakietów z packets.js
    if (typeof packets === 'undefined') return;

    // Unikaj duplikatów jeśli id jest dostępne
    if (packet.id && packets.find(p => p.id === packet.id)) return;

    // Dodaj na początek
    packets.unshift(packet);
    if (packets.length > 100) packets.pop();

    if (packet.id && typeof lastPacketId !== 'undefined') {
        lastPacketId = Math.max(lastPacketId, packet.id);
    }

    // Odśwież widok
    if (typeof renderPackets === 'function') {
        renderPackets(packets);
    }
}
