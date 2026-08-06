#!/usr/bin/env node
/**
 * Captures one dashboard screenshot per formula for the Formula Reference.
 *
 * Each image shows the Evaluate card of the live dashboard: the request JSON
 * on the left of the fold, the response the server actually returned below it.
 * Nothing here is mocked -- if the server errors, the run fails rather than
 * writing a screenshot of a broken request.
 *
 * Usage:
 *   node scripts/capture-formula-screenshots.js <base-url> <api-key> [name...]
 *
 * With no trailing names every formula in CASES is captured. Pass one or more
 * formula names to refresh just those.
 *
 * Requires a running server and puppeteer-core:
 *   npm install puppeteer-core@23 --no-save
 *
 * Chrome writes truecolour PNGs, roughly 25 KB each. The reference stores
 * palette-indexed ones at about 8 KB. These are flat-colour UI captures, so
 * quantising to 256 colours only shifts antialiased text edges. After a
 * capture run, match the existing images with:
 *
 *   python3 -c "
 *   from PIL import Image; import sys
 *   for p in sys.argv[1:]:
 *       Image.open(p).convert('RGB').quantize(colors=256, dither=Image.NONE).save(p, optimize=True)
 *   " docs/img/formulas/<name>.png
 */

const fs = require("fs");
const path = require("path");
const puppeteer = require("puppeteer-core");

const BASE = process.argv[2] || "http://localhost:8080";
const API_KEY = process.argv[3] || "demo-key";
const ONLY = process.argv.slice(4);

const OUT_DIR = path.join(__dirname, "..", "docs", "img", "formulas");

// Chrome ships under several names depending on the distribution.
const CHROME_CANDIDATES = [
  process.env.CHROME_PATH,
  "/usr/bin/google-chrome",
  "/usr/bin/chromium",
  "/usr/bin/chromium-browser",
].filter(Boolean);

/**
 * One entry per formula. Values are chosen to be financially sensible and to
 * make the result recognisable to a reader -- a bond priced at par, an EAR
 * that is visibly above its nominal rate -- rather than merely in-domain.
 */
const CASES = {
  net_present_value: {
    formula: "net_present_value",
    rate: 0.1,
    cash_flows: [-1000, 300, 400, 500, 600],
  },
  internal_rate_of_return: {
    formula: "internal_rate_of_return",
    cash_flows: [-1000, 500, 500, 500],
  },
  annuity_present_value: {
    formula: "annuity_present_value",
    payment: 100,
    rate: 0.05,
    periods: 10,
  },
  annuity_future_value: {
    formula: "annuity_future_value",
    payment: 100,
    rate: 0.05,
    periods: 10,
  },
  perpetuity_present_value: {
    formula: "perpetuity_present_value",
    payment: 100,
    rate: 0.05,
  },
  effective_annual_rate: {
    formula: "effective_annual_rate",
    nominal_rate: 0.12,
    compounding_periods: 12,
  },
  return_on_assets: {
    formula: "return_on_assets",
    net_income: 150000,
    avg_total_assets: 2000000,
  },
  dupont_roe: {
    formula: "dupont_roe",
    profit_margin: 0.15,
    asset_turnover: 2.0,
    equity_multiplier: 2.0,
  },
  current_ratio: {
    formula: "current_ratio",
    current_assets: 500000,
    current_liabilities: 250000,
  },
  debt_to_equity: {
    formula: "debt_to_equity",
    total_liabilities: 400000,
    shareholders_equity: 1000000,
  },
  net_interest_margin: {
    formula: "net_interest_margin",
    interest_income: 500000,
    interest_expense: 200000,
    avg_earning_assets: 10000000,
  },
  loan_to_deposit_ratio: {
    formula: "loan_to_deposit_ratio",
    total_loans: 800000,
    total_deposits: 1000000,
  },
  sharpe_ratio: {
    formula: "sharpe_ratio",
    portfolio_return: 0.12,
    risk_free_rate: 0.02,
    portfolio_std_dev: 0.15,
  },
  jensens_alpha: {
    formula: "jensens_alpha",
    portfolio_return: 0.12,
    risk_free_rate: 0.02,
    market_return: 0.1,
    beta: 1.2,
  },
  dividend_discount_model: {
    formula: "dividend_discount_model",
    next_dividend: 2.0,
    required_return: 0.1,
    growth_rate: 0.04,
  },
  bond_price: {
    formula: "bond_price",
    face_value: 1000,
    coupon_payment: 50,
    yield_per_period: 0.05,
    periods: 10,
  },
  free_cash_flow_to_firm: {
    formula: "free_cash_flow_to_firm",
    ebit: 500000,
    tax_rate: 0.21,
    depreciation: 100000,
    delta_working_capital: 50000,
    capex: 150000,
  },
};

function chromePath() {
  for (const candidate of CHROME_CANDIDATES) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }
  throw new Error(
    `no Chrome found; tried ${CHROME_CANDIDATES.join(", ")}. Set CHROME_PATH.`,
  );
}

function requestBody(kind) {
  return {
    nodes: [{ formula: { name: "result", kind } }],
    edges: [],
    inputs: {},
  };
}

async function main() {
  const names = ONLY.length > 0 ? ONLY : Object.keys(CASES);
  for (const name of names) {
    if (!CASES[name]) {
      throw new Error(`unknown formula '${name}'`);
    }
  }

  fs.mkdirSync(OUT_DIR, { recursive: true });

  const browser = await puppeteer.launch({
    executablePath: chromePath(),
    headless: "new",
    args: ["--no-sandbox", "--disable-dev-shm-usage"],
  });

  const failures = [];
  try {
    const page = await browser.newPage();
    await page.setViewport({ width: 1280, height: 1400, deviceScaleFactor: 1 });

    // Surface page-side errors instead of silently shipping a broken capture.
    page.on("pageerror", (err) => failures.push(`page exception: ${err.message}`));

    await page.goto(`${BASE}/dashboard`, { waitUntil: "networkidle0" });
    await page.$eval("#apiKey", (el, key) => {
      el.value = key;
    }, API_KEY);

    for (const name of names) {
      const body = JSON.stringify(requestBody(CASES[name]), null, 2);

      await page.$eval("#evaluateJson", (el, text) => {
        el.value = text;
      }, body);
      await page.$eval("#evaluateOut", (el) => {
        el.textContent = "";
      });

      await page.click("#evaluateBtn");

      // Wait for the server's answer to land, then confirm it is a result and
      // not an error rendered into the same element.
      try {
        await page.waitForFunction(
          () => document.getElementById("evaluateOut").textContent.trim().length > 0,
          { timeout: 15000 },
        );
      } catch {
        failures.push(`${name}: no response within 15s`);
        continue;
      }

      const output = await page.$eval("#evaluateOut", (el) => el.textContent);
      if (!output.includes("outputs")) {
        failures.push(`${name}: server did not return a result -> ${output.trim().slice(0, 160)}`);
        continue;
      }

      // Crop the chart out. The reference pairs each entry with its request
      // and the number that came back; the bar chart of a single scalar adds
      // nothing, and the 45 entries captured before this script existed do not
      // show it. Only the framing changes -- the request and response in shot
      // are exactly what the server was sent and returned.
      await page.$eval(".chart-wrap", (el) => {
        el.style.display = "none";
      });

      const card = await page.evaluateHandle(() =>
        [...document.querySelectorAll("section.card")].find(
          (s) => s.querySelector("h2")?.textContent.trim() === "Evaluate",
        ),
      );
      await card.asElement().screenshot({
        path: path.join(OUT_DIR, `${name}.png`),
      });

      const value = JSON.parse(output).outputs.result;
      process.stdout.write(`  ${name.padEnd(26)} ${value}\n`);
    }
  } finally {
    await browser.close();
  }

  if (failures.length > 0) {
    process.stderr.write(`\n${failures.length} failure(s):\n  ${failures.join("\n  ")}\n`);
    process.exit(1);
  }
  process.stdout.write(`\ncaptured ${names.length} screenshot(s) into ${OUT_DIR}\n`);
}

main().catch((err) => {
  process.stderr.write(`${err.stack || err}\n`);
  process.exit(1);
});
