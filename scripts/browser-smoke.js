#!/usr/bin/env node
//
// Browser smoke test for the CASIROS dashboard.
//
// Loads the dashboard in headless Chrome and drives it the way a person would:
// enter an API key, check health, run an evaluation, run a simulation. Fails on
// any uncaught page error, failed asset request, or missing result.
//
// This covers a class of defect the Rust tests structurally cannot see. The
// dashboard once shipped with a chart handler that threw on every click
// ("window[canvasId].destroy is not a function") and, separately, with relative
// asset paths that 404'd unless the URL carried a trailing slash — leaving the
// page unstyled. Both are invisible to any test that does not run a browser.
//
// Usage:
//   node scripts/browser-smoke.js [baseUrl] [apiKey]
//
// Environment:
//   CHROME_PATH   path to a Chrome/Chromium binary (default: /usr/bin/google-chrome)

const puppeteer = require('puppeteer-core');

const BASE = process.argv[2] || process.env.CASIROS_BASE_URL || 'http://localhost:8080';
const KEY = process.argv[3] || process.env.CASIROS_API_KEY || 'demo-key';
const CHROME = process.env.CHROME_PATH || '/usr/bin/google-chrome';

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const failures = [];

function check(condition, message) {
  if (condition) {
    console.log(`  ok    ${message}`);
  } else {
    console.log(`  FAIL  ${message}`);
    failures.push(message);
  }
}

(async () => {
  console.log(`browser smoke: ${BASE}`);

  const browser = await puppeteer.launch({
    executablePath: CHROME,
    args: ['--no-sandbox', '--disable-gpu', '--hide-scrollbars'],
    defaultViewport: { width: 1280, height: 1000 },
  });

  try {
    const page = await browser.newPage();

    // Any of these means the page is broken even if it looks fine.
    const pageErrors = [];
    const failedRequests = [];
    // Browsers request /favicon.ico unprompted. The app does not ship one, so
    // its 404/401 says nothing about whether the dashboard works.
    const ignorable = (url) => new URL(url).pathname === '/favicon.ico';

    page.on('pageerror', (e) => pageErrors.push(e.message));
    page.on('requestfailed', (r) => {
      if (!ignorable(r.url())) failedRequests.push(`${r.url()} ${r.failure()?.errorText}`);
    });
    page.on('response', (r) => {
      const sameOrigin = new URL(r.url()).origin === new URL(BASE).origin;
      if (r.status() >= 400 && sameOrigin && !ignorable(r.url())) {
        failedRequests.push(`${r.status()} ${r.url()}`);
      }
    });

    // --- Load ---------------------------------------------------------------
    // No trailing slash: this is the form that exposed the relative-asset bug.
    const resp = await page.goto(`${BASE}/dashboard`, { waitUntil: 'networkidle2' });
    check(resp.ok(), `GET /dashboard returned ${resp.status()}`);

    // --- Styles actually applied --------------------------------------------
    // An unstyled page still "loads"; the giveaway is the body background
    // falling back to the browser default instead of the dashboard's dark theme.
    const bg = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
    check(
      bg !== 'rgba(0, 0, 0, 0)' && bg !== 'rgb(255, 255, 255)',
      `stylesheet applied (body background ${bg})`
    );

    // --- Scripts wired up ----------------------------------------------------
    const wired = await page.evaluate(() => typeof window.Chart !== 'undefined');
    check(wired, 'chart library loaded');

    // --- Health check --------------------------------------------------------
    await page.type('#apiKey', KEY);
    await page.click('#healthBtn');
    await page.waitForFunction(
      () => document.getElementById('healthOut').textContent.includes('status'),
      { timeout: 15000 }
    );
    const health = await page.$eval('#healthOut', (el) => el.textContent);
    check(health.includes('ok'), 'health check returned ok');

    // --- Evaluate ------------------------------------------------------------
    await page.click('#evaluateBtn');
    await page.waitForFunction(
      () => document.getElementById('evaluateOut').textContent.trim().length > 0,
      { timeout: 20000 }
    );
    const evalOut = await page.$eval('#evaluateOut', (el) => el.textContent);
    check(evalOut.includes('outputs'), 'evaluation returned outputs');
    check(!/is not a function|undefined|Error/i.test(evalOut), 'evaluation produced no error text');

    // The chart must actually render — this is what the destroy() bug broke.
    const charted = await page.evaluate(() => {
      const c = document.getElementById('evaluateChart');
      return c && c.width > 0 && c.height > 0;
    });
    check(charted, 'evaluate chart rendered');

    // --- Simulate ------------------------------------------------------------
    await page.click('#simulateBtn');
    await page.waitForFunction(
      () => document.getElementById('simulateOut').textContent.trim().length > 0,
      { timeout: 30000 }
    );
    const simOut = await page.$eval('#simulateOut', (el) => el.textContent);
    check(simOut.includes('mean'), 'simulation returned statistics');

    // --- Re-run to exercise chart teardown -----------------------------------
    // The original defect only fired on the *second* render, when the code
    // tried to destroy the previous chart.
    await page.click('#evaluateBtn');
    await sleep(2000);
    const rerun = await page.$eval('#evaluateOut', (el) => el.textContent);
    check(rerun.includes('outputs'), 're-running evaluation still succeeds');

    // --- Nothing threw -------------------------------------------------------
    check(pageErrors.length === 0, `no uncaught page errors${pageErrors.length ? `: ${pageErrors.join('; ')}` : ''}`);
    check(
      failedRequests.length === 0,
      `no failed requests${failedRequests.length ? `: ${failedRequests.join('; ')}` : ''}`
    );
  } finally {
    await browser.close();
  }

  if (failures.length) {
    console.error(`\nbrowser smoke FAILED (${failures.length}):`);
    for (const f of failures) console.error(`  - ${f}`);
    process.exit(1);
  }
  console.log('\nbrowser smoke passed');
})().catch((e) => {
  console.error(`browser smoke ERROR: ${e.message}`);
  process.exit(1);
});
