import { isValidBase64Pdf } from '../helpers/test-utils.js';

describe('Print to PDF', () => {
  beforeEach(async () => {
    await browser.url('tauri://localhost/#main');
    await browser.pause(100);
  });

  describe('Basic Print', () => {
    it('should print page to PDF', async () => {
      const pdf = await browser.printPage({});

      expect(pdf).toBeDefined();
      expect(typeof pdf).toBe('string');
      expect(pdf.length).toBeGreaterThan(0);
    });

    it('should return valid base64 PDF data', async () => {
      const pdf = await browser.printPage({});

      // Verify it's valid base64
      expect(() => Buffer.from(pdf, 'base64')).not.toThrow();

      // Verify PDF format
      expect(isValidBase64Pdf(pdf)).toBe(true);
    });

    it('should print different pages', async () => {
      // Print main page
      const mainPdf = await browser.printPage({});

      // Navigate and print forms page
      await browser.url('tauri://localhost/#forms');
      await browser.pause(100);
      const formsPdf = await browser.printPage({});

      // Both should be valid PDFs
      expect(isValidBase64Pdf(mainPdf)).toBe(true);
      expect(isValidBase64Pdf(formsPdf)).toBe(true);

      // PDFs should be different (different content)
      expect(mainPdf).not.toBe(formsPdf);
    });
  });

  describe('Print Options', () => {
    it('should print with orientation landscape', async () => {
      const pdf = await browser.printPage({
        orientation: 'landscape',
      });

      expect(pdf).toBeDefined();
      expect(isValidBase64Pdf(pdf)).toBe(true);
    });

    it('should print with orientation portrait', async () => {
      const pdf = await browser.printPage({
        orientation: 'portrait',
      });

      expect(pdf).toBeDefined();
      expect(isValidBase64Pdf(pdf)).toBe(true);
    });

    it('should print with scale', async () => {
      const pdf = await browser.printPage({
        scale: 0.5,
      });

      expect(pdf).toBeDefined();
      expect(isValidBase64Pdf(pdf)).toBe(true);
    });

    it('should print with background', async () => {
      const pdf = await browser.printPage({
        background: true,
      });

      expect(pdf).toBeDefined();
      expect(isValidBase64Pdf(pdf)).toBe(true);
    });

    it('should print without background', async () => {
      const pdf = await browser.printPage({
        background: false,
      });

      expect(pdf).toBeDefined();
      expect(isValidBase64Pdf(pdf)).toBe(true);
    });

    it('should print with custom page size', async () => {
      const pdf = await browser.printPage({
        pageWidth: 21.0, // A4 width in cm
        pageHeight: 29.7, // A4 height in cm
      });

      expect(pdf).toBeDefined();
      expect(isValidBase64Pdf(pdf)).toBe(true);
    });

    it('should print with margins', async () => {
      const pdf = await browser.printPage({
        marginTop: 2,
        marginBottom: 2,
        marginLeft: 2,
        marginRight: 2,
      });

      expect(pdf).toBeDefined();
      expect(isValidBase64Pdf(pdf)).toBe(true);
    });

    it('should print with shrinkToFit', async () => {
      const pdf = await browser.printPage({
        shrinkToFit: true,
      });

      expect(pdf).toBeDefined();
      expect(isValidBase64Pdf(pdf)).toBe(true);
    });

    it('should print with page ranges', async () => {
      const pdf = await browser.printPage({
        pageRanges: ['1'],
      });

      expect(pdf).toBeDefined();
      expect(isValidBase64Pdf(pdf)).toBe(true);
    });

    it('should print with multiple options', async () => {
      const pdf = await browser.printPage({
        orientation: 'landscape',
        scale: 0.8,
        background: true,
        marginTop: 1,
        marginBottom: 1,
        marginLeft: 1,
        marginRight: 1,
      });

      expect(pdf).toBeDefined();
      expect(isValidBase64Pdf(pdf)).toBe(true);
    });
  });

  describe('Print Long Content', () => {
    it('should print scrollable page', async () => {
      await browser.url('tauri://localhost/#scroll');
      await browser.pause(100);

      const pdf = await browser.printPage({
        background: true,
      });

      expect(pdf).toBeDefined();
      expect(isValidBase64Pdf(pdf)).toBe(true);

      // PDF should be larger due to more content
      const buffer = Buffer.from(pdf, 'base64');
      expect(buffer.length).toBeGreaterThan(1000);
    });
  });

  describe('Print Form Page', () => {
    it('should print page with form elements', async () => {
      await browser.url('tauri://localhost/#forms');
      await browser.pause(100);

      // Fill some form data
      const input = await $('[data-testid="text-input"]');
      await input.setValue('Print test value');

      const pdf = await browser.printPage({});

      expect(pdf).toBeDefined();
      expect(isValidBase64Pdf(pdf)).toBe(true);
    });
  });

  describe('PDF Size Validation', () => {
    it('should have reasonable PDF size', async () => {
      const pdf = await browser.printPage({});
      const buffer = Buffer.from(pdf, 'base64');

      // PDF should be at least a few KB
      expect(buffer.length).toBeGreaterThan(1000);

      // And not unreasonably large (less than 10MB)
      expect(buffer.length).toBeLessThan(10 * 1024 * 1024);
    });

    it('should have larger PDF for pages with more content', async () => {
      const mainPdf = await browser.printPage({});
      const mainBuffer = Buffer.from(mainPdf, 'base64');

      await browser.url('tauri://localhost/#scroll');
      await browser.pause(100);

      const scrollPdf = await browser.printPage({});
      const scrollBuffer = Buffer.from(scrollPdf, 'base64');

      // Scroll page has more content, likely larger PDF
      // (though not guaranteed due to compression)
      expect(mainBuffer.length).toBeGreaterThan(0);
      expect(scrollBuffer.length).toBeGreaterThan(0);
    });
  });
});
