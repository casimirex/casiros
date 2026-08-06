#!/usr/bin/env node
/**
 * Captures the dashboard screenshots used in README.md.
 *
 * Drives the real dashboard in headless Chrome and photographs it doing work:
 * health checked, an evaluation run, a simulation run with its histogram, and
 * the audit trail listing the requests the earlier shots just made. Nothing is
 * mocked, and the script fails rather than writing an image if a panel reports
 * an error, so a screenshot here cannot show a request that did not work.
 *
 * Usage:
 *   node scripts/capture-dashboard-screenshots.js <base-url> <api-key>
 *
 * Requires a running server and puppeteer-core:
 *   npm install puppeteer-core@23 --no-save
 *
 * Images are written truecolour. Match the repository's palette-indexed
 * convention afterwards:
 *
 *   python3 -c "
 *   from PIL import Image; import sys
 *   for p in sys.argv[1:]:
 *       Image.open(p).convert('RGB').quantize(colors=256, dither=Image.NONE).save(p, optimize=True)
 *   " docs/img/readme-*.png
 */

const fs = require("fs");
const path = require("path");
const puppeteer = require("puppeteer-core");

const BASE = process.argv[2] || "http://localhost:8080";
const API_KEY = process.argv[3] || "demo-key";
const OUT_DIR = path.join(__dirname, "..", "docs", "img");

const CHROME_CANDIDATES = [
  process.env.CHROME_PATH,
  "/usr/bin/google-chrome",
  "/usr/bin/chromium",
  "/usr/bin/chromium-browser",
].filter(Boolean);

function chromePath() {
  for (const c of CHROME_CANDIDATES) {
    if (fs.existsSync(c)) return c;
  }
  throw new Error(`no Chrome found; tried ${CHROME_CANDIDATES.join(", ")}`);
}

/** Clicks a button and waits for its output element to fill in. */
async function run(page, button, output, label, failures) {
  await page.$eval(output, (el) => {
    el.textContent = "";
  });
  await page.click(button);
  try {
    await page.waitForFunction(
      (sel) => document.querySelector(sel).textContent.trim().length > 0,
      { timeout: 20000 },
      output,
    );
  } catch {
    failures.push(`${label}: no response within 20s`);
    return "";
  }
  // The dashboard marks the outcome with a CSS class rather than anything in
  // the text. Matching on the word "error" instead would flag a perfectly good
  // audit response, whose events each carry an `error_message` field.
  const { text, failed } = await page.$eval(output, (el) => ({
    text: el.textContent,
    failed: el.classList.contains("error"),
  }));
  if (failed) {
    failures.push(`${label}: panel reported an error -> ${text.trim().slice(0, 160)}`);
  }
  return text;
}

/** Screenshots the card whose <h2> matches `heading`. */
async function shotCard(page, heading, file) {
  const card = await page.evaluateHandle(
    (h) =>
      [...document.querySelectorAll("section.card")].find(
        (s) => s.querySelector("h2")?.textContent.trim() === h,
      ),
    heading,
  );
  const el = card.asElement();
  if (!el) throw new Error(`card not found: ${heading}`);
  await el.screenshot({ path: path.join(OUT_DIR, file) });
  process.stdout.write(`  ${file}\n`);
}

async function main() {
  fs.mkdirSync(OUT_DIR, { recursive: true });

  const browser = await puppeteer.launch({
    executablePath: chromePath(),
    headless: "new",
    args: ["--no-sandbox", "--disable-dev-shm-usage"],
  });

  const failures = [];
  try {
    const page = await browser.newPage();
    page.on("pageerror", (e) => failures.push(`page exception: ${e.message}`));
    page.on("requestfailed", (r) =>
      failures.push(`asset failed: ${r.url().split("/").pop()}`),
    );

    await page.setViewport({ width: 1100, height: 1400, deviceScaleFactor: 1 });
    await page.goto(`${BASE}/dashboard`, { waitUntil: "networkidle0" });

    await page.$eval("#apiKey", (el, k) => {
      el.value = k;
    }, API_KEY);

    // Drive each panel in turn. The audit panel runs last so it lists the
    // requests the earlier panels actually made.
    await run(page, "#healthBtn", "#healthOut", "health", failures);
    const evaluated = await run(page, "#evaluateBtn", "#evaluateOut", "evaluate", failures);
    const simulated = await run(page, "#simulateBtn", "#simulateOut", "simulate", failures);

    // Chart.js animates; let it settle before photographing.
    await new Promise((r) => setTimeout(r, 1200));

    await shotCard(page, "Evaluate", "readme-evaluate.png");
    await shotCard(page, "Simulate", "readme-simulate.png");

    // Two events is enough to show the shape of the trail. The default ten
    // produces an image taller than most README readers will scroll.
    await page.$eval("#auditLimit", (el) => {
      el.value = "2";
    });
    await run(page, "#auditBtn", "#auditOut", "audit", failures);
    await new Promise((r) => setTimeout(r, 400));
    await shotCard(page, "Audit", "readme-audit.png");

    // Full-page hero, with every panel above showing real output.
    await page.evaluate(() => window.scrollTo(0, 0));
    await page.screenshot({
      path: path.join(OUT_DIR, "readme-dashboard.png"),
      fullPage: false,
    });
    process.stdout.write("  readme-dashboard.png\n");

    // Swagger UI, served from the same binary.
    await page.goto(`${BASE}/swagger-ui/`, { waitUntil: "networkidle0" });
    await new Promise((r) => setTimeout(r, 1500));
    await page.screenshot({ path: path.join(OUT_DIR, "readme-swagger.png") });
    process.stdout.write("  readme-swagger.png\n");

    process.stdout.write(
      `\nevaluate -> ${JSON.parse(evaluated).outputs.fv.slice(0, 18)}…\n` +
        `simulate -> count ${JSON.parse(simulated).count}, mean ${JSON.parse(simulated).mean.slice(0, 10)}…\n`,
    );
  } finally {
    await browser.close();
  }

  if (failures.length > 0) {
    process.stderr.write(`\n${failures.length} failure(s):\n  ${failures.join("\n  ")}\n`);
    process.exit(1);
  }
  process.stdout.write(`\nwrote screenshots into ${OUT_DIR}\n`);
}

main().catch((err) => {
  process.stderr.write(`${err.stack || err}\n`);
  process.exit(1);
});
