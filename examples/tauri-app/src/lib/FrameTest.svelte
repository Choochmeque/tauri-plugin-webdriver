<script>
  const iframeContent = `
    <!DOCTYPE html>
    <html>
      <head>
        <style>
          body { font-family: sans-serif; padding: 1rem; }
          button { padding: 0.5rem 1rem; margin: 0.5rem; }
        </style>
      </head>
      <body>
        <div id="frame-content" data-testid="frame-content">Inside Frame</div>
        <p data-testid="frame-text">This is content inside the iframe.</p>
        <button id="frame-button" data-testid="frame-button">Frame Button</button>
        <input id="frame-input" data-testid="frame-input" placeholder="Frame input..." />
      </body>
    </html>
  `;
</script>

<section data-testid="frame-test-page">
  <h1 data-testid="frame-page-heading">Frame Test Page</h1>

  <p data-testid="frame-page-description">This page tests iframe switching functionality.</p>

  <div data-testid="frame-container" class="frame-container">
    <h2>Primary Frame</h2>
    <iframe
      id="test-frame"
      data-testid="test-frame"
      srcdoc={iframeContent}
      title="Test Frame"
      width="400"
      height="200"
    ></iframe>
  </div>

  <div data-testid="frame-container-2" class="frame-container">
    <h2>Secondary Frame</h2>
    <iframe
      id="test-frame-2"
      data-testid="test-frame-2"
      srcdoc={iframeContent.replace('Inside Frame', 'Inside Frame 2').replace('Frame Button', 'Frame 2 Button')}
      title="Test Frame 2"
      width="400"
      height="200"
    ></iframe>
  </div>

  <div data-testid="outside-frame">
    <p>This content is outside the frames.</p>
    <button data-testid="outside-button">Outside Button</button>
  </div>
</section>

<style>
  .frame-container {
    margin: 1rem 0;
    padding: 1rem;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
  }

  .frame-container h2 {
    margin-top: 0;
    font-size: 1rem;
  }

  iframe {
    border: 1px solid rgba(255, 255, 255, 0.3);
    background: white;
  }
</style>
