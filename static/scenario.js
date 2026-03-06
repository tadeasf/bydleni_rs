// scenario.js — My Budget Scenario (localStorage-backed, no account needed)
(function() {
    const STORAGE_KEY = 'bydleni_scenario';

    const FIELDS = [
        { key: 'net_income', id: 'sc-net-income' },
        { key: 'current_savings', id: 'sc-savings' },
        { key: 'flat_size_m2', id: 'sc-flat-size' },
        { key: 'mortgage_rate_pct', id: 'sc-rate' },
        { key: 'ltv_pct', id: 'sc-ltv' },
        { key: 'mortgage_years', id: 'sc-years' },
        { key: 'monthly_expenses', id: 'sc-expenses' },
        { key: 'investment_return_pct', id: 'sc-return' },
    ];

    function getScenario() {
        try {
            const raw = localStorage.getItem(STORAGE_KEY);
            return raw ? JSON.parse(raw) : null;
        } catch { return null; }
    }

    function saveScenario(data) {
        try { localStorage.setItem(STORAGE_KEY, JSON.stringify(data)); } catch {}
    }

    function clearScenario() {
        try { localStorage.removeItem(STORAGE_KEY); } catch {}
    }

    function readFormData() {
        const data = {};
        for (const f of FIELDS) {
            const el = document.getElementById(f.id);
            if (el) data[f.key] = el.value;
        }
        return data;
    }

    function populateForm(data) {
        if (!data) return;
        for (const f of FIELDS) {
            const el = document.getElementById(f.id);
            if (el && data[f.key] !== undefined && data[f.key] !== '') {
                el.value = data[f.key];
            }
        }
    }

    function showReset(visible) {
        const btn = document.getElementById('scenario-reset');
        if (btn) btn.style.display = visible ? 'block' : 'none';
    }

    function updateSummaryStrip(data) {
        const el = document.getElementById('scenario-summary');
        if (!el) return;
        if (!data || !data.net_income) {
            el.innerHTML = '';
            return;
        }
        // Post to summary endpoint
        const params = new URLSearchParams();
        for (const [k, v] of Object.entries(data)) {
            params.append(k, v);
        }
        fetch('/api/scenario/summary', {
            method: 'POST',
            headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
            body: params,
        })
            .then(r => r.text())
            .then(html => { el.innerHTML = html; });
    }

    // Validate client-side (mirrors server)
    function validateInput(data) {
        const errors = [];
        const income = parseFloat(data.net_income);
        if (!income || income <= 0) errors.push('Net income must be positive');
        const savings = parseFloat(data.current_savings);
        if (savings < 0) errors.push('Savings cannot be negative');
        const size = parseFloat(data.flat_size_m2);
        if (!size || size <= 0 || size > 500) errors.push('Flat size must be 1-500 m\u00b2');
        const rate = parseFloat(data.mortgage_rate_pct);
        if (rate < 0 || rate > 30) errors.push('Rate must be 0-30%');
        const ltv = parseFloat(data.ltv_pct);
        if (!ltv || ltv <= 0 || ltv > 100) errors.push('LTV must be 1-100%');
        const years = parseInt(data.mortgage_years);
        if (!years || years <= 0 || years > 50) errors.push('Term must be 1-50 years');
        const expenses = parseFloat(data.monthly_expenses);
        if (expenses < 0) errors.push('Expenses cannot be negative');
        return errors;
    }

    function showErrors(errors) {
        const el = document.getElementById('scenario-errors');
        if (!el) return;
        if (errors.length === 0) { el.innerHTML = ''; return; }
        el.innerHTML = '<div class="scenario-errors">' + errors.join('; ') + '</div>';
    }

    // Region page: prefill mortgage/savings calculators from scenario
    function prefillRegionCalculators() {
        const data = getScenario();
        if (!data) return;

        // Mortgage calculator
        const rateEl = document.getElementById('rate');
        const yearsEl = document.getElementById('years');
        const ltvEl = document.getElementById('ltv');
        if (rateEl && data.mortgage_rate_pct) rateEl.value = data.mortgage_rate_pct;
        if (yearsEl && data.mortgage_years) yearsEl.value = data.mortgage_years;
        if (ltvEl && data.ltv_pct) ltvEl.value = data.ltv_pct;

        // Savings calculator
        const netIncomeEl = document.querySelector('input[name="net_income"]');
        const expensesEl = document.querySelector('input[name="expenses"]');
        const returnEl = document.querySelector('input[name="return_pct"]');
        if (netIncomeEl && data.net_income) netIncomeEl.value = data.net_income;
        if (expensesEl && data.monthly_expenses) expensesEl.value = data.monthly_expenses;
        if (returnEl && data.investment_return_pct) returnEl.value = data.investment_return_pct;

        // Trigger scenario-detail
        const detailTarget = document.getElementById('scenario-detail');
        if (detailTarget) {
            const regionSlug = detailTarget.dataset.region;
            if (regionSlug) {
                const params = new URLSearchParams();
                params.append('region', regionSlug);
                for (const [k, v] of Object.entries(data)) {
                    params.append(k, v);
                }
                fetch('/api/scenario/region-detail', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
                    body: params,
                })
                    .then(r => r.text())
                    .then(html => { detailTarget.innerHTML = html; });
            }
        }
    }

    // Compare page: show toggle + handle hidden form
    function initComparePage() {
        const data = getScenario();
        const toggle = document.getElementById('scenario-compare-toggle');
        if (!toggle || !data) return;

        toggle.style.display = 'inline-flex';
        toggle.addEventListener('click', () => {
            const active = toggle.classList.toggle('active');
            const tbody = document.getElementById('scenario-compare-body');
            const defaultBody = document.getElementById('default-compare-body');
            const theadDefault = document.getElementById('compare-thead-default');
            const theadScenario = document.getElementById('compare-thead-scenario');
            if (active && tbody) {
                const params = new URLSearchParams();
                for (const [k, v] of Object.entries(data)) {
                    params.append(k, v);
                }
                fetch('/api/scenario/compare', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
                    body: params,
                })
                    .then(r => r.text())
                    .then(html => {
                        tbody.innerHTML = html;
                        tbody.style.display = '';
                        if (defaultBody) defaultBody.style.display = 'none';
                        if (theadDefault) theadDefault.style.display = 'none';
                        if (theadScenario) theadScenario.style.display = '';
                    });
            } else {
                if (tbody) { tbody.innerHTML = ''; tbody.style.display = 'none'; }
                if (defaultBody) defaultBody.style.display = '';
                if (theadDefault) theadDefault.style.display = '';
                if (theadScenario) theadScenario.style.display = 'none';
            }
        });
    }

    // Main init
    document.addEventListener('DOMContentLoaded', () => {
        const form = document.getElementById('scenario-form');

        // Index page: restore + auto-submit
        if (form) {
            const saved = getScenario();
            if (saved) {
                populateForm(saved);
                showReset(true);
                updateSummaryStrip(saved);
                // Auto-submit via HTMX
                setTimeout(() => htmx.trigger(form, 'submit'), 100);
            }

            // Save on submit
            document.body.addEventListener('htmx:beforeRequest', (e) => {
                if (e.detail.elt === form || e.detail.elt.closest('#scenario-form')) {
                    const data = readFormData();
                    const errors = validateInput(data);
                    if (errors.length > 0) {
                        showErrors(errors);
                        e.preventDefault();
                        return;
                    }
                    showErrors([]);
                    saveScenario(data);
                    showReset(true);
                    updateSummaryStrip(data);
                }
            });

            // Reset handler
            const resetBtn = document.getElementById('scenario-reset');
            if (resetBtn) {
                resetBtn.addEventListener('click', () => {
                    clearScenario();
                    form.reset();
                    showReset(false);
                    const summary = document.getElementById('scenario-summary');
                    if (summary) summary.innerHTML = '';
                    const results = document.getElementById('scenario-regions');
                    if (results) results.innerHTML = '';
                    const errors = document.getElementById('scenario-errors');
                    if (errors) errors.innerHTML = '';
                });
            }
        }

        // Region page
        prefillRegionCalculators();

        // Compare page
        initComparePage();
    });
})();
