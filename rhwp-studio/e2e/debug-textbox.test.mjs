import { launchBrowser, loadApp, closeBrowser } from './helpers.mjs';
async function test() {
  const browser = await launchBrowser();
  const page = await browser.newPage();
  await page.setViewport({ width: 1280, height: 900 });
  await loadApp(page);
  await page.evaluate(() => window.__eventBus?.emit('create-new-document'));
  await page.evaluate(() => new Promise(r => setTimeout(r, 1000)));

  const result = await page.evaluate(() => {
    const w = window.__wasm;
    w.doc.insertText(0, 0, 0, 'Before');
    w.doc.splitParagraph(0, 0, 6);
    w.doc.splitParagraph(0, 1, 0);
    w.doc.insertText(0, 2, 0, 'After');

    const tb = JSON.parse(w.doc.createShapeControl(JSON.stringify({
      sectionIdx: 0, paraIdx: 1, charOffset: 0,
      width: 21600, height: 7200,
      shapeType: 'textbox', textWrap: 'TopAndBottom',
    })));

    const svg = w.doc.renderPageSvg(0);

    // SVG에서 y 좌표 추출 (text 요소)
    const textElements = svg.match(/<text[^>]+y="([^"]+)"[^>]*>([^<])<\/text>/g) || [];
    const positions = textElements.map(t => {
      const yM = t.match(/y="([^"]+)"/);
      const chM = t.match(/>([^<])</);
      return { y: yM?.[1], ch: chM?.[1] };
    });

    // rect 요소 (글상자 테두리)
    const rects = svg.match(/<rect[^/]*\/>/g) || [];

    return { positions: positions.slice(0, 20), rects: rects.slice(0, 5), svgLen: svg.length };
  });
  console.log(JSON.stringify(result, null, 2));
  await page.close();
  await closeBrowser(browser);
}
test().catch(console.error);
