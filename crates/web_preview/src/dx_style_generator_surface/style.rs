const DX_STYLE_GENERATOR_CSS: &str = r##"    :root {
      color-scheme: dark;
      --bg: #090b10;
      --panel: #111827;
      --panel-soft: #172033;
      --text: #f8fafc;
      --muted: #94a3b8;
      --border: #293548;
      --accent: #38bdf8;
      --accent-strong: #22c55e;
      --warning: #f59e0b;
    }
    * { box-sizing: border-box; }
    html, body { margin: 0; min-height: 100%; background: var(--bg); color: var(--text); font-family: Inter, ui-sans-serif, system-ui, sans-serif; }
    body { padding: 14px; }
    main { display: grid; grid-template-columns: minmax(220px, 280px) minmax(0, 1fr); gap: 12px; min-height: calc(100vh - 28px); }
    aside, section { border: 1px solid var(--border); background: var(--panel); border-radius: 8px; min-width: 0; }
    aside { overflow: auto; }
    header { padding: 14px; border-bottom: 1px solid var(--border); }
    h1 { margin: 0 0 6px; font-size: 17px; line-height: 1.2; }
    h2 { margin: 0; font-size: 13px; color: var(--muted); font-weight: 500; }
    button, select, input { font: inherit; }
    .catalog-search { display: grid; gap: 6px; padding: 10px 12px; border-bottom: 1px solid var(--border); color: var(--muted); font-size: 11px; }
    .catalog-search input { width: 100%; background: #0d1320; color: var(--text); border: 1px solid var(--border); border-radius: 6px; padding: 7px 8px; }
    .catalog { padding: 8px; display: grid; gap: 6px; }
    .generator { width: 100%; border: 1px solid transparent; background: transparent; color: var(--text); border-radius: 6px; padding: 8px; text-align: left; cursor: pointer; }
    .generator:hover, .generator[aria-current="true"] { background: var(--panel-soft); border-color: var(--border); }
    .generator strong { display: block; font-size: 12px; }
    .generator span { display: block; color: var(--muted); font-size: 11px; margin-top: 2px; }
    .catalog-empty { color: var(--muted); font-size: 12px; padding: 8px; }
    .workspace { display: grid; grid-template-rows: auto minmax(220px, 1fr) auto; overflow: hidden; }
    .toolbar { display: flex; gap: 8px; align-items: center; padding: 12px; border-bottom: 1px solid var(--border); background: #0d1320; }
    .toolbar select { min-width: 180px; background: var(--panel); color: var(--text); border: 1px solid var(--border); border-radius: 6px; padding: 7px 9px; }
    .toolbar button { background: var(--panel); color: var(--text); border: 1px solid var(--border); border-radius: 6px; padding: 7px 10px; }
    .toolbar button:disabled { color: var(--muted); cursor: not-allowed; opacity: .72; }
    .status { margin-left: auto; color: var(--muted); font-size: 12px; }
    .canvas { display: grid; grid-template-columns: minmax(240px, 360px) minmax(0, 1fr); gap: 12px; padding: 12px; overflow: auto; }
    .controls { display: grid; align-content: start; gap: 10px; }
    .control { display: grid; gap: 6px; color: var(--muted); font-size: 12px; }
    .control input, .control select { width: 100%; background: #0d1320; color: var(--text); border: 1px solid var(--border); border-radius: 6px; padding: 7px 8px; }
    .preview-wrap { display: grid; gap: 10px; min-width: 0; }
    .preview { min-height: 260px; border: 1px solid var(--border); border-radius: 8px; background: #0f172a; display: grid; place-items: center; padding: 22px; overflow: hidden; }
    .sample { width: min(420px, 80%); min-height: 160px; border-radius: 18px; display: grid; place-items: center; color: white; font-weight: 700; letter-spacing: 0; text-align: center; padding: 24px; background: linear-gradient(120deg, #38bdf8, #22c55e); box-shadow: 0 24px 80px rgba(56,189,248,.28); }
    .sample[data-preview-kind="layout-items"] { width: min(520px, 92%); align-items: stretch; justify-items: stretch; }
    .preview-item { display: grid; place-items: center; min-height: 54px; border-radius: 8px; background: rgba(255,255,255,.14); border: 1px solid rgba(255,255,255,.22); }
    .sample[data-preview-kind="timeline"] { gap: 14px; }
    .timeline-track { width: min(320px, 90%); height: 8px; border-radius: 999px; background: rgba(255,255,255,.18); display: flex; justify-content: space-between; align-items: center; padding: 0 4px; }
    .timeline-track span { width: 16px; height: 16px; border-radius: 999px; background: white; box-shadow: 0 0 0 4px rgba(255,255,255,.14); }
    .timeline-label, .preview-subtitle { color: rgba(255,255,255,.78); font-size: 12px; font-weight: 500; }
    .sample[data-preview-kind="swatch-pair"] { gap: 12px; }
    .swatch-row { display: grid; grid-template-columns: repeat(2, minmax(80px, 1fr)); gap: 12px; width: min(260px, 90%); }
    .swatch-row span { min-height: 76px; border-radius: 10px; border: 1px solid rgba(255,255,255,.24); box-shadow: inset 0 1px 0 rgba(255,255,255,.2); }
    .sample[data-preview-kind="text-card"] { gap: 8px; align-content: center; }
    .preview-title { font-size: 30px; line-height: 1.1; }
    @keyframes dx-style-pulse { 0% { transform: scale(.94); opacity: .72; } 50% { transform: scale(1.04); opacity: 1; } 100% { transform: scale(.94); opacity: .72; } }
    .patch-review { display: grid; gap: 6px; border: 1px solid var(--border); border-radius: 8px; background: #0d1320; padding: 10px; color: var(--muted); font-size: 12px; }
    .patch-review strong { color: var(--text); font-size: 12px; }
    .patch-review dl { display: grid; grid-template-columns: max-content minmax(0, 1fr); gap: 4px 10px; margin: 0; }
    .patch-review dd { margin: 0; color: var(--text); word-break: break-word; }
    .patch-review ul { margin: 2px 0 0; padding-left: 18px; color: var(--text); }
    .patch-review li { margin: 2px 0; word-break: break-word; }
    pre { margin: 0; white-space: pre-wrap; word-break: break-word; background: #050812; border: 1px solid var(--border); border-radius: 8px; padding: 10px; color: #dbeafe; font-size: 12px; line-height: 1.45; }
    footer { display: flex; gap: 8px; align-items: center; padding: 10px 12px; border-top: 1px solid var(--border); color: var(--muted); font-size: 12px; }
    .pill { border: 1px solid var(--border); border-radius: 999px; padding: 3px 8px; color: var(--muted); }
    .ready { color: var(--accent-strong); }
    .blocked { color: var(--warning); }
    @media (max-width: 780px) {
      main, .canvas { grid-template-columns: 1fr; }
      aside { max-height: 220px; }
    }
"##;

pub(super) fn dx_style_generator_css() -> &'static str {
    DX_STYLE_GENERATOR_CSS
}
