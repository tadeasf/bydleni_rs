// Bydleni.rs — Chart.js helpers (FT editorial style)

function isLight() {
    return document.documentElement.classList.contains('light');
}

function palette() {
    const light = isLight();
    return {
        text: light ? '#66605a' : '#726859',
        grid: light ? '#e0d5c5' : '#211f1b',
        tooltipBg: light ? '#1a1714' : '#f2ece0',
        tooltipText: light ? '#f2ece0' : '#1a1714',
        salmon: light ? '#c4532a' : '#ffb899',
    };
}

// Set Chart.js defaults for FT style
Chart.defaults.font.family = "'Source Sans 3', 'Source Sans Pro', system-ui, sans-serif";
Chart.defaults.font.size = 11;
Chart.defaults.color = '#726859';
Chart.defaults.plugins.tooltip.backgroundColor = '#1a1714';
Chart.defaults.plugins.tooltip.titleColor = '#ffb899';
Chart.defaults.plugins.tooltip.bodyColor = '#f2ece0';
Chart.defaults.plugins.tooltip.borderColor = '#2a2722';
Chart.defaults.plugins.tooltip.borderWidth = 1;
Chart.defaults.plugins.tooltip.cornerRadius = 4;
Chart.defaults.plugins.tooltip.padding = { top: 8, bottom: 8, left: 12, right: 12 };
Chart.defaults.plugins.tooltip.titleFont = { family: "'JetBrains Mono', monospace", weight: '700', size: 11 };
Chart.defaults.plugins.tooltip.bodyFont = { family: "'JetBrains Mono', monospace", size: 12 };
Chart.defaults.plugins.tooltip.displayColors = false;

function applyTheme(chart) {
    const p = palette();
    const scales = chart.options.scales || {};
    for (const axis of Object.values(scales)) {
        if (axis.ticks) axis.ticks.color = p.text;
        if (axis.grid) axis.grid.color = p.grid;
    }
    const tt = chart.options.plugins?.tooltip;
    if (tt) {
        tt.backgroundColor = p.tooltipBg;
        tt.titleColor = p.salmon;
        tt.bodyColor = p.tooltipText;
    }
    chart.update('none');
}

// Listen for theme changes
window.addEventListener('themechange', () => {
    Chart.instances && Object.values(Chart.instances).forEach(c => applyTheme(c));
});

async function loadBarChart(canvasId, endpoint) {
    const canvas = document.getElementById(canvasId);
    if (!canvas) return;
    let resp, data;
    try {
        resp = await fetch(endpoint);
        data = await resp.json();
    } catch (e) {
        console.error(`Failed to load chart ${canvasId}:`, e);
        return;
    }
    const p = palette();

    new Chart(canvas, {
        type: 'bar',
        data: {
            labels: data.labels,
            datasets: data.datasets.map(ds => ({
                label: ds.label,
                data: ds.data,
                backgroundColor: ds.backgroundColor || p.salmon,
                borderRadius: 3,
                borderSkipped: false,
                maxBarThickness: 48,
            })),
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            animation: {
                duration: 800,
                easing: 'easeOutQuart',
            },
            plugins: {
                legend: { display: false },
                tooltip: {
                    callbacks: {
                        label: (ctx) => {
                            const v = ctx.parsed.y;
                            return v >= 1000
                                ? Math.round(v).toLocaleString('cs-CZ') + ' ' + (ctx.dataset.label || '')
                                : v.toFixed(0) + ' ' + (ctx.dataset.label || '');
                        },
                    },
                },
            },
            scales: {
                x: {
                    ticks: {
                        color: p.text,
                        font: { size: 10, weight: '600' },
                        maxRotation: 45,
                        autoSkip: false,
                    },
                    grid: { display: false },
                    border: { display: false },
                },
                y: {
                    ticks: {
                        color: p.text,
                        font: { family: "'JetBrains Mono', monospace", size: 10 },
                        callback: (v) => v >= 1000 ? (v / 1000).toFixed(0) + 'k' : v,
                    },
                    grid: { color: p.grid, lineWidth: 0.5 },
                    border: { display: false },
                },
            },
        },
    });
}

async function loadForecastChart(canvasId, endpoint, opts = {}) {
    const canvas = document.getElementById(canvasId);
    if (!canvas) return;
    let resp, data;
    try {
        resp = await fetch(endpoint);
        data = await resp.json();
    } catch (e) {
        console.error(`Failed to load forecast chart ${canvasId}:`, e);
        return;
    }
    if (!data.labels || data.labels.length === 0) return;
    const p = palette();
    const dualAxis = data.datasets.some(ds => ds.yAxisID === 'y1');

    const scales = {
        x: {
            ticks: { color: p.text, font: { size: 10 }, maxRotation: 0 },
            grid: { display: false },
            border: { display: false },
        },
        y: {
            position: 'left',
            title: {
                display: true,
                text: opts.yLabel || 'Index',
                color: p.text,
                font: { size: 10 },
            },
            ticks: {
                color: p.text,
                font: { family: "'JetBrains Mono', monospace", size: 10 },
            },
            grid: { color: p.grid, lineWidth: 0.5 },
            border: { display: false },
        },
    };

    if (dualAxis) {
        scales.y1 = {
            position: 'right',
            title: {
                display: true,
                text: 'Years',
                color: p.text,
                font: { size: 10 },
            },
            ticks: {
                color: p.text,
                font: { family: "'JetBrains Mono', monospace", size: 10 },
            },
            grid: { drawOnChartArea: false },
            border: { display: false },
        };
    }

    new Chart(canvas, {
        type: 'line',
        data: {
            labels: data.labels,
            datasets: data.datasets.map(ds => ({
                label: ds.label,
                data: ds.data,
                borderColor: ds.borderColor || p.salmon,
                backgroundColor: 'transparent',
                fill: false,
                tension: 0.3,
                pointRadius: ds.borderDash ? 0 : 3,
                pointHitRadius: 10,
                pointHoverRadius: 4,
                pointBackgroundColor: ds.borderColor || p.salmon,
                pointHoverBackgroundColor: ds.borderColor || p.salmon,
                pointHoverBorderColor: '#0d0b07',
                pointHoverBorderWidth: 2,
                borderWidth: 2,
                borderDash: ds.borderDash || [],
                spanGaps: false,
                yAxisID: ds.yAxisID || 'y',
            })),
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            animation: { duration: 800, easing: 'easeOutQuart' },
            interaction: { mode: 'index', intersect: false },
            plugins: {
                legend: {
                    display: true,
                    labels: {
                        color: p.text,
                        font: { size: 9, weight: '600' },
                        boxWidth: 16,
                        boxHeight: 2,
                        padding: 10,
                        filter: (item) => !item.text.includes('projected'),
                    },
                },
                tooltip: {
                    callbacks: {
                        label: (ctx) => {
                            if (ctx.parsed.y == null) return '';
                            const v = ctx.parsed.y;
                            return ctx.dataset.label + ': ' + v.toFixed(1);
                        },
                    },
                },
            },
            scales,
        },
    });
}

async function loadLineChart(canvasId, endpoint) {
    const canvas = document.getElementById(canvasId);
    if (!canvas) return;
    let resp, data;
    try {
        resp = await fetch(endpoint);
        data = await resp.json();
    } catch (e) {
        console.error(`Failed to load line chart ${canvasId}:`, e);
        return;
    }
    const p = palette();

    new Chart(canvas, {
        type: 'line',
        data: {
            labels: data.labels,
            datasets: data.datasets.map(ds => ({
                label: ds.label,
                data: ds.data,
                borderColor: ds.borderColor || p.salmon,
                backgroundColor: (ds.borderColor || p.salmon) + '15',
                fill: true,
                tension: 0.35,
                pointRadius: 0,
                pointHitRadius: 12,
                pointHoverRadius: 4,
                pointHoverBackgroundColor: ds.borderColor || p.salmon,
                pointHoverBorderColor: '#0d0b07',
                pointHoverBorderWidth: 2,
                borderWidth: 2,
            })),
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            animation: {
                duration: 1000,
                easing: 'easeOutQuart',
            },
            interaction: { mode: 'index', intersect: false },
            plugins: {
                legend: {
                    display: data.datasets.length > 1,
                    labels: {
                        color: p.text,
                        font: { size: 10, weight: '600' },
                        boxWidth: 12,
                        boxHeight: 2,
                        padding: 16,
                    },
                },
                tooltip: {
                    callbacks: {
                        label: (ctx) => {
                            const v = ctx.parsed.y;
                            return v >= 1000
                                ? Math.round(v).toLocaleString('cs-CZ')
                                : v.toFixed(2);
                        },
                    },
                },
            },
            scales: {
                x: {
                    ticks: {
                        color: p.text,
                        font: { size: 10 },
                        maxTicksLimit: 8,
                        maxRotation: 0,
                    },
                    grid: { display: false },
                    border: { display: false },
                },
                y: {
                    ticks: {
                        color: p.text,
                        font: { family: "'JetBrains Mono', monospace", size: 10 },
                        callback: (v) => v >= 1000 ? (v / 1000).toFixed(0) + 'k' : v,
                    },
                    grid: { color: p.grid, lineWidth: 0.5 },
                    border: { display: false },
                },
            },
        },
    });
}
