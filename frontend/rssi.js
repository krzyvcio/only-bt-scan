// RSSI - Wykresy i analiza RSSI
// ═══════════════════════════════════════════════════════════════════════════

function loadRssiTrend(deviceMac) {
    const chartContainer = document.getElementById('rssi-chart');
    if (!chartContainer) return;

    chartContainer.innerHTML = `
        <div class="loading-spinner">
            <div class="spinner"></div>
            <span>Loading RSSI trend data (last 24h)...</span>
        </div>
    `;

    fetch(`/api/devices/${encodeURIComponent(deviceMac)}/rssi-24h`)
        .then(response => {
            if (!response.ok) throw new Error(`HTTP ${response.status}`);
            return response.json();
        })
        .then(data => {
            if (data.success && data.measurements && data.measurements.length > 0) {
                drawRssiChart(data.measurements, chartContainer);

                const avgRssi = parseFloat(data.rssi_stats.avg).toFixed(1);
                document.getElementById('rssi-avg').textContent = `${avgRssi} dBm`;
                document.getElementById('rssi-count').textContent = data.measurements_count;

                const trendDir = data.trend.direction;
                let trendEmoji = '➡️';
                if (trendDir === 'getting_closer') trendEmoji = '📶 Zbliża się';
                else if (trendDir === 'getting_farther') trendEmoji = '📉 Oddala się';
                else trendEmoji = '➡️ Stabilne';

                document.getElementById('rssi-trend').textContent = trendEmoji;
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

    const rssiValues = measurements.map(m => m.rssi);
    const minRssi = Math.min(...rssiValues);
    const maxRssi = Math.max(...rssiValues);
    const rssiRange = maxRssi - minRssi || 10;

    const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    svg.setAttribute('width', '100%');
    svg.setAttribute('height', '100%');
    svg.setAttribute('viewBox', `0 0 ${width} ${height}`);
    svg.style.cssText = 'display: block;';

    // Grid lines
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

        const label = document.createElementNS('http://www.w3.org/2000/svg', 'text');
        label.setAttribute('x', padding - 10);
        label.setAttribute('y', y + 4);
        label.setAttribute('text-anchor', 'end');
        label.setAttribute('font-size', '12');
        label.setAttribute('fill', '#8b949e');
        label.textContent = `${Math.round(rssiLevel)}`;
        svg.appendChild(label);
    }

    // Line path
    let pathD = '';
    measurements.forEach((m, idx) => {
        const x = padding + (chartWidth / (measurements.length - 1 || 1)) * idx;
        const normalizedRssi = (m.rssi - minRssi) / rssiRange;
        const y = padding + chartHeight - (normalizedRssi * chartHeight);

        pathD += idx === 0 ? `M ${x} ${y}` : ` L ${x} ${y}`;
    });

    const path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    path.setAttribute('d', pathD);
    path.setAttribute('stroke', '#58a6ff');
    path.setAttribute('stroke-width', '2');
    path.setAttribute('fill', 'none');
    path.setAttribute('stroke-linecap', 'round');
    path.setAttribute('stroke-linejoin', 'round');
    svg.appendChild(path);

    // Points
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

    // Axes
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

    // Time labels
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
            timeLabel.textContent = new Date(m.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
            svg.appendChild(timeLabel);
        }
    }

    container.innerHTML = '';
    container.appendChild(svg);
}

function drawQualityBars(distribution) {
    const container = document.getElementById('quality-bars');
    if (!container) return;

    if (!distribution) {
        container.innerHTML = '<p class="empty-msg">Brak danych o jakości sygnału</p>';
        return;
    }

    const total = Object.values(distribution).reduce((a, b) => a + b, 0) || 1;
    const qualities = ['excellent', 'good', 'fair', 'poor', 'very_poor'];
    const colors = ['#3fb950', '#58a6ff', '#d29922', '#f85149', '#d62828'];

    let html = '<div class="quality-bars-container">';
    qualities.forEach((q, i) => {
        const count = distribution[q] || 0;
        const pct = (count / total * 100).toFixed(1);
        html += `
            <div class="quality-bar-item">
                <span class="quality-label">${q}</span>
                <div class="quality-bar-track">
                    <div class="quality-bar-fill" style="width: ${pct}%; background: ${colors[i]};"></div>
                </div>
                <span class="quality-count">${count}</span>
            </div>
        `;
    });
    html += '</div>';
    container.innerHTML = html;
}
