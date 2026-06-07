// app.js
// Simulates the asynchronous WebSockets/FFI bridge connecting to the Zeus Backend

const valRpm = document.getElementById('val-rpm');
const valHeat = document.getElementById('val-heat');
const valThroughput = document.getElementById('val-throughput');
const consoleOutput = document.getElementById('console-output');
const chartRpm = document.getElementById('chart-rpm');
const chartHeat = document.getElementById('chart-heat');

// Initialize charts with empty bars
const NUM_BARS = 30;
for (let i = 0; i < NUM_BARS; i++) {
    const bar = document.createElement('div');
    bar.className = 'chart-bar';
    chartRpm.appendChild(bar);
    
    const barHeat = document.createElement('div');
    barHeat.className = 'chart-bar';
    chartHeat.appendChild(barHeat);
}

// Simulated FFI Bridge from Zeus `analytics_engine.zs`
class ZeusFFIBridge {
    constructor() {
        this.baseRpm = 4500;
        this.baseHeat = 85.5;
        this.baseThroughput = 12.5; // GB/s
        this.isRunning = false;
    }

    startStream() {
        this.isRunning = true;
        // High-frequency telemetry loop (simulating M:N fiber execution speed)
        setInterval(() => this.pollZeusBackend(), 50); 
    }

    pollZeusBackend() {
        if (!this.isRunning) return;

        // Simulate Zeus parallel {} crunching
        const noiseRpm = (Math.random() - 0.5) * 500;
        const noiseHeat = (Math.random() - 0.5) * 5;
        const noiseThroughput = (Math.random() - 0.2) * 1.5;

        const currentRpm = Math.max(0, this.baseRpm + noiseRpm);
        const currentHeat = Math.max(0, this.baseHeat + noiseHeat);
        const currentThroughput = Math.max(10, this.baseThroughput + noiseThroughput);

        this.updateUI(currentRpm, currentHeat, currentThroughput);
        this.logToConsole(currentRpm, currentHeat);
    }

    updateUI(rpm, heat, throughput) {
        // Update text values
        valRpm.innerText = Math.floor(rpm).toLocaleString();
        valHeat.innerText = heat.toFixed(2);
        valThroughput.innerText = `${throughput.toFixed(2)} GB/s`;

        // Update charts (shift left)
        this.updateChart(chartRpm, (rpm / 8000) * 100, '#3b82f6');
        this.updateChart(chartHeat, (heat / 150) * 100, '#ef4444');
    }

    updateChart(chartElement, percentage, color) {
        const bars = chartElement.children;
        // Shift heights left
        for (let i = 0; i < bars.length - 1; i++) {
            bars[i].style.height = bars[i+1].style.height;
            bars[i].style.backgroundColor = bars[i+1].style.backgroundColor;
        }
        // Add new height to the rightmost bar
        const newBar = bars[bars.length - 1];
        const clampedPct = Math.min(100, Math.max(5, percentage));
        newBar.style.height = `${clampedPct}%`;
        newBar.style.backgroundColor = color;
    }

    logToConsole(rpm, heat) {
        // Only log 10% of the time so we don't flood the DOM instantly
        if (Math.random() > 0.1) return;

        const now = new Date();
        const timestamp = `${now.getHours().toString().padStart(2,'0')}:${now.getMinutes().toString().padStart(2,'0')}:${now.getSeconds().toString().padStart(2,'0')}.${now.getMilliseconds().toString().padStart(3,'0')}`;
        
        const line = document.createElement('div');
        line.className = 'log-line';
        line.innerHTML = `<span class="log-timestamp">[${timestamp}]</span><span class="log-data">ZEUS_FFI: Frame Processed | RPM: ${Math.floor(rpm)} | Jitter: < 1ns</span>`;
        
        consoleOutput.appendChild(line);
        
        // Auto-scroll
        consoleOutput.scrollTop = consoleOutput.scrollHeight;

        // Keep console log clean (max 50 lines)
        if (consoleOutput.childElementCount > 50) {
            consoleOutput.removeChild(consoleOutput.firstChild);
        }
    }
}

// Boot the Zeus Engine Hybrid Link
document.addEventListener('DOMContentLoaded', () => {
    const bridge = new ZeusFFIBridge();
    bridge.startStream();
});
