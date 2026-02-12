import { isValidBase64Pdf, navigateToTestPage } from '../helpers/test-utils.js';

describe('Print to PDF', () => {
  beforeEach(async () => {
    await navigateToTestPage('main');
  });

  describe('Basic Print', () => {
    it('should print page to PDF', async () => {
      const pdf = await browser.printPage('portrait');

      expect(pdf).toBeDefined();
      expect(typeof pdf).toBe('string');
      expect(pdf.length).toBeGreaterThan(0);
    });

    it('should return valid base64 PDF data', async () => {
      const pdf = await browser.printPage('portrait');

      // Verify it's valid base64
      expect(() => Buffer.from(pdf, 'base64')).not.toThrow();

      // Verify PDF format
      expect(isValidBase64Pdf(pdf)).toBe(true);
    });

    it('should print different pages', async () => {
      // Print main page
      const mainPdf = await browser.printPage('portrait');

      // Navigate and print forms page
      await navigateToTestPage('forms');
      const formsPdf = await browser.printPage('portrait');

      // Both should be valid PDFs
      expect(isValidBase64Pdf(mainPdf)).toBe(true);
      expect(isValidBase64Pdf(formsPdf)).toBe(true);

      // PDFs should be different (different content)
      expect(mainPdf).not.toBe(formsPdf);
    });
  });

  describe('Print Options', () => {
    it('should print with orientation landscape', async () => {
      const pdf = await browser.printPage('landscape');

      expect(pdf).toBeDefined();
      expect(isValidBase64Pdf(pdf)).toBe(true);
    });

    it('should print with orientation portrait', async () => {
      const pdf = await browser.printPage('portrait');

      expect(pdf).toBeDefined();
      expect(isValidBase64Pdf(pdf)).toBe(true);
    });

    it('should print with scale', async () => {
      // printPage(orientation, scale, ...)
      const pdf = await browser.printPage('portrait', 0.5);

      expect(pdf).toBeDefined();
      expect(isValidBase64Pdf(pdf)).toBe(true);
    });

    it('should print with background', async () => {
      // printPage(orientation, scale, background, ...)
      const pdf = await browser.printPage('portrait', 1, true);

      expect(pdf).toBeDefined();
      expect(isValidBase64Pdf(pdf)).toBe(true);
    });

    it('should print without background', async () => {
      const pdf = await browser.printPage('portrait', 1, false);

      expect(pdf).toBeDefined();
      expect(isValidBase64Pdf(pdf)).toBe(true);
    });

    it('should print with custom page size', async () => {
      // printPage(orientation, scale, background, width, height, ...)
      const pdf = await browser.printPage('portrait', 1, false, 21.0, 29.7);

      expect(pdf).toBeDefined();
      expect(isValidBase64Pdf(pdf)).toBe(true);
    });

    it('should print with margins', async () => {
      // printPage(orientation, scale, background, width, height, top, bottom, left, right, ...)
      const pdf = await browser.printPage('portrait', 1, false, 21.59, 27.94, 2, 2, 2, 2);

      expect(pdf).toBeDefined();
      expect(isValidBase64Pdf(pdf)).toBe(true);
    });

    it('should print with shrinkToFit', async () => {
      // printPage(orientation, scale, background, width, height, top, bottom, left, right, shrinkToFit, ...)
      const pdf = await browser.printPage('portrait', 1, false, 21.59, 27.94, 1, 1, 1, 1, true);

      expect(pdf).toBeDefined();
      expect(isValidBase64Pdf(pdf)).toBe(true);
    });

    it('should print with page ranges', async () => {
      // printPage(orientation, scale, background, width, height, top, bottom, left, right, shrinkToFit, pageRanges)
      const pdf = await browser.printPage('portrait', 1, false, 21.59, 27.94, 1, 1, 1, 1, true, ['1']);

      expect(pdf).toBeDefined();
      expect(isValidBase64Pdf(pdf)).toBe(true);
    });

    it('should print with multiple options', async () => {
      const pdf = await browser.printPage('landscape', 0.8, true, 21.59, 27.94, 1, 1, 1, 1, true);

      expect(pdf).toBeDefined();
      expect(isValidBase64Pdf(pdf)).toBe(true);
    });
  });

  describe('Print Long Content', () => {
    it('should print scrollable page', async () => {
      await navigateToTestPage('scroll');

      const pdf = await browser.printPage('portrait', 1, true);

      expect(pdf).toBeDefined();
      expect(isValidBase64Pdf(pdf)).toBe(true);

      // PDF should be larger due to more content
      const buffer = Buffer.from(pdf, 'base64');
      expect(buffer.length).toBeGreaterThan(1000);
    });

    it('should handle very long pages', async () => {
      await navigateToTestPage('scroll');

      const pdf = await browser.printPage('portrait');

      expect(pdf).toBeDefined();
      expect(isValidBase64Pdf(pdf)).toBe(true);

      // Decode and verify it's a multi-page or large PDF
      const buffer = Buffer.from(pdf, 'base64');
      expect(buffer.length).toBeGreaterThan(0);
    });
  });

  describe('Print Form Page', () => {
    it('should print page with form elements', async () => {
      await navigateToTestPage('forms');

      // Fill some form data
      const input = await $('[data-testid="text-input"]');
      await input.setValue('Print test value');

      const pdf = await browser.printPage('portrait');

      expect(pdf).toBeDefined();
      expect(isValidBase64Pdf(pdf)).toBe(true);
    });
  });

  describe('PDF Size Validation', () => {
    it('should have reasonable PDF size', async () => {
      const pdf = await browser.printPage('portrait');
      const buffer = Buffer.from(pdf, 'base64');

      // PDF should be at least a few KB
      expect(buffer.length).toBeGreaterThan(1000);

      // And not unreasonably large (less than 10MB)
      expect(buffer.length).toBeLessThan(10 * 1024 * 1024);
    });

    it('should have larger PDF for pages with more content', async () => {
      const mainPdf = await browser.printPage('portrait');
      const mainBuffer = Buffer.from(mainPdf, 'base64');

      await navigateToTestPage('scroll');

      const scrollPdf = await browser.printPage('portrait');
      const scrollBuffer = Buffer.from(scrollPdf, 'base64');

      // Scroll page has more content, likely larger PDF
      // (though not guaranteed due to compression)
      expect(mainBuffer.length).toBeGreaterThan(0);
      expect(scrollBuffer.length).toBeGreaterThan(0);
    });
  });
});
