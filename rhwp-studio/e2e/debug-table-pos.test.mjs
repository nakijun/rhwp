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
    w.doc.insertText(0, 0, 0, 'Before table');
    w.doc.splitParagraph(0, 0, 12);
    w.doc.splitParagraph(0, 1, 0);  // 빈 문단 (표 호스트)
    w.doc.insertText(0, 2, 0, 'After table');

    const tr = JSON.parse(w.doc.createTable(0, 1, 0, 2, 2));
    w.doc.insertTextInCell(0, tr.paraIdx, tr.controlIdx, 0, 0, 0, 'Cell');

    window.__eventBus?.emit('document-changed');

    const svg = w.doc.renderPageSvg(0);

    // SVG에서 text 요소의 y 좌표 추출
    const textElements = svg.match(/<text[^>]+y="([^"]+)"[^>]*>([^<])<\/text>/g) || [];
    const positions = textElements.map(t => {
      const yM = t.match(/y="([^"]+)"/);
      const chM = t.match(/>([^<])</);
      return { y: yM?.[1], ch: chM?.[1] };
    });

    // 표 rect (셀 테두리)
    const rects = svg.match(/<rect[^/]*\/>/g) || [];
    const tableRects = rects.filter(r => r.includes('stroke'));

    return { positions: positions.slice(0, 20), tableRects: tableRects.slice(0, 5), svgLen: svg.length };
  });
  console.log(JSON.stringify(result, null, 2));
  await page.close();
  await closeBrowser(browser);
}
test().catch(console.error);
