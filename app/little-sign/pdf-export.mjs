// Map the displayed page back to its original PDF coordinates, including rotation and cropping.
export async function buildSignedPdf(PDFLib, bytes, placements, views) {
  const doc = await PDFLib.PDFDocument.load(bytes.slice());
  const cache = new Map();
  for (const p of placements) {
    if (!cache.has(p.url)) cache.set(p.url, await doc.embedPng(p.url));
    const image = cache.get(p.url);
    const page = doc.getPage(p.page - 1);
    const v = views.get(p.page);
    const [a, b, c, d, e, f] = v.transform;
    const det = a * d - b * c;
    const inverse = [d / det, -b / det, -c / det, a / det, (c * f - d * e) / det, (b * e - a * f) / det];
    const name = page.node.newXObject('PlacedItem', image.ref);
    page.pushOperators(
      PDFLib.pushGraphicsState(),
      PDFLib.concatTransformationMatrix(...inverse),
      PDFLib.concatTransformationMatrix(p.w * v.width, 0, 0, -p.h * v.height, p.x * v.width, (p.y + p.h) * v.height),
      PDFLib.drawObject(name),
      PDFLib.popGraphicsState()
    );
  }
  return doc.save();
}
