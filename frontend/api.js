// ═══════════════════════════════════════════════════════════════════════════
// API - Funkcje do komunikacji z backendem
// ═══════════════════════════════════════════════════════════════════════════

const API_BASE = '/api';
const POLL_INTERVAL = 2000;
const PACKET_POLL_INTERVAL = 1000;

async function apiGet(endpoint) {
    try {
        const response = await fetch(`${API_BASE}${endpoint}`);
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        return await response.json();
    } catch (error) {
        console.error(`API Error: ${endpoint}`, error);
        return null;
    }
}

async function loadDevicesFromApi() {
    return await apiGet('/devices?page=1&limit=50');
}

async function loadPacketsFromApi() {
    return await apiGet('/raw-packets?limit=100');
}

async function loadTelemetryFromApi() {
    return await apiGet('/rssi-telemetry');
}

async function loadDeviceHistory(mac) {
    return await apiGet(`/devices/${encodeURIComponent(mac)}/history`);
}

async function loadRssiTrendData(mac) {
    return await apiGet(`/devices/${encodeURIComponent(mac)}/rssi-24h`);
}

async function loadStatsFromApi() {
    return await apiGet('/stats');
}

async function loadCompanyIdsStats() {
    return await apiGet('/company-ids/stats');
}

async function loadDataFlowStats() {
    return await apiGet('/data-flow-stats');
}

async function loadEventAnalyzerStats() {
    return await apiGet('/event-analyzer-stats');
}

async function loadTemporalCorrelations() {
    return await apiGet('/temporal-correlations');
}

async function loadDeviceAnomalies(mac) {
    return await apiGet(`/devices/${encodeURIComponent(mac)}/anomalies`);
}

async function loadDeviceBehavior(mac) {
    return await apiGet(`/devices/${encodeURIComponent(mac)}/behavior`);
}
