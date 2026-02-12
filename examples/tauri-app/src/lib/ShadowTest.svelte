<script>
  import { onMount } from 'svelte';

  let shadowHost;
  let shadowHostOpen;
  let shadowHostClosed;

  onMount(() => {
    // Create open shadow DOM
    if (shadowHostOpen) {
      const shadow = shadowHostOpen.attachShadow({ mode: 'open' });
      shadow.innerHTML = `
        <style>
          .shadow-content {
            padding: 1rem;
            background: rgba(100, 149, 237, 0.2);
            border-radius: 4px;
            margin: 0.5rem 0;
          }
          button {
            padding: 0.5rem 1rem;
            margin: 0.5rem;
          }
          input {
            padding: 0.5rem;
            margin: 0.5rem;
          }
        </style>
        <div id="shadow-content" data-testid="shadow-content" class="shadow-content">
          <p data-testid="shadow-text">Inside Open Shadow DOM</p>
          <button id="shadow-button" data-testid="shadow-button">Shadow Button</button>
          <input id="shadow-input" data-testid="shadow-input" placeholder="Shadow input..." />
          <div data-testid="nested-shadow-container">
            <span data-testid="nested-shadow-span">Nested element in shadow</span>
          </div>
        </div>
      `;
    }

    // Create closed shadow DOM (for testing that closed shadows are not accessible)
    if (shadowHostClosed) {
      const closedShadow = shadowHostClosed.attachShadow({ mode: 'closed' });
      closedShadow.innerHTML = `
        <div data-testid="closed-shadow-content">
          <p>Inside Closed Shadow DOM - Should not be accessible via WebDriver</p>
        </div>
      `;
    }
  });
</script>

<section data-testid="shadow-test-page">
  <h1 data-testid="shadow-page-heading">Shadow DOM Test Page</h1>

  <p data-testid="shadow-page-description">This page tests Shadow DOM functionality.</p>

  <div class="shadow-section">
    <h2>Open Shadow DOM</h2>
    <div
      bind:this={shadowHostOpen}
      data-testid="shadow-host-open"
      id="shadow-host-open"
      class="shadow-host"
    >
      <!-- Shadow DOM will be attached here -->
    </div>
  </div>

  <div class="shadow-section">
    <h2>Closed Shadow DOM</h2>
    <div
      bind:this={shadowHostClosed}
      data-testid="shadow-host-closed"
      id="shadow-host-closed"
      class="shadow-host"
    >
      <!-- Closed Shadow DOM will be attached here -->
    </div>
    <p class="note">Note: Closed shadow DOMs should not be accessible via WebDriver.</p>
  </div>

  <div data-testid="regular-content" class="regular-section">
    <h2>Regular DOM Content</h2>
    <p data-testid="regular-text">This is regular DOM content (not in shadow DOM).</p>
    <button data-testid="regular-button">Regular Button</button>
  </div>
</section>

<style>
  .shadow-section {
    margin: 1rem 0;
    padding: 1rem;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
  }

  .shadow-section h2 {
    margin-top: 0;
    font-size: 1rem;
  }

  .shadow-host {
    min-height: 50px;
    border: 2px dashed rgba(255, 255, 255, 0.3);
    padding: 0.5rem;
  }

  .regular-section {
    margin: 1rem 0;
    padding: 1rem;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
  }

  .regular-section h2 {
    margin-top: 0;
    font-size: 1rem;
  }

  .note {
    font-size: 0.8rem;
    color: rgba(255, 255, 255, 0.6);
    font-style: italic;
  }
</style>
