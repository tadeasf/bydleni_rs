# Frontend Architecture

## Stack

- **Askama** templates (Jinja2-like, compiled at build time)
- **HTMX** for interactive form submissions without full page reloads
- **Chart.js** for data visualization
- **Vanilla JS** for map interaction and scenario management

## Template hierarchy

```
templates/
  base.html          -- nav, footer, theme toggle, script includes
  index.html         -- homepage (map, charts, scenario panel, data table)
  pages/
    region.html      -- region detail (metrics, calculators, charts, listings)
    compare.html     -- comparison table + charts
    404.html         -- not found page
```

## CSS design system

FT (Financial Times) editorial-inspired design with:
- Dark mode default, light mode toggle
- CSS custom properties for all colors
- Heat scale (`--heat-1` to `--heat-5`) for affordability severity
- Salmon accent color (`--salmon`) throughout
- JetBrains Mono for numeric values
- Playfair Display for headings

## JavaScript

- `charts.js` -- Chart.js wrappers (`loadBarChart`, `loadLineChart`, `loadForecastChart`), dark mode support
- `scenario.js` -- My Budget Scenario (localStorage, form prefill, auto-submit, cross-page sync)
