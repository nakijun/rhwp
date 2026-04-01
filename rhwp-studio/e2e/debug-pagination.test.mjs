import { launchBrowser, loadApp, clickEditArea, closeBrowser } from './helpers.mjs';
async function test() {
  const browser = await launchBrowser();
  const page = await browser.newPage();
  await page.setViewport({ width: 1280, height: 900 });
  await loadApp(page);
  await page.evaluate(() => window.__eventBus?.emit('create-new-document'));
  await page.evaluate(() => new Promise(r => setTimeout(r, 1000)));
  await clickEditArea(page);

  const before = await page.evaluate(() => ({
    pageCount: window.__wasm?.pageCount,
    paraCount: window.__wasm?.getParagraphCount(0),
  }));
  console.log('Before:', before);

  // 50줄 Enter로 문단 생성
  for (let i = 0; i < 50; i++) {
    await page.keyboard.type('Line ' + i, { delay: 5 });
    await page.keyboard.press('Enter');
  }
  await page.evaluate(() => new Promise(r => setTimeout(r, 1000)));

  const after = await page.evaluate(() => ({
    pageCount: window.__wasm?.pageCount,
    paraCount: window.__wasm?.getParagraphCount(0),
    canvasCount: document.querySelectorAll('canvas').length,
    scrollH: document.querySelector('#scroll-container')?.scrollHeight,
  }));
  console.log('After:', after);

  await page.close();
  await closeBrowser(browser);
}
test().catch(console.error);
