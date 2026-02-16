<script>
  import Greet from './lib/Greet.svelte'
  import FrameTest from './lib/FrameTest.svelte'
  import ShadowTest from './lib/ShadowTest.svelte'
  import AlertTest from './lib/AlertTest.svelte'
  import ScrollTest from './lib/ScrollTest.svelte'
  import FormTest from './lib/FormTest.svelte'

  let currentRoute = $state(window.location.hash || '#main')

  // Listen for hash changes
  $effect(() => {
    const handleHashChange = () => {
      currentRoute = window.location.hash || '#main'
    }
    window.addEventListener('hashchange', handleHashChange)
    return () => window.removeEventListener('hashchange', handleHashChange)
  })
</script>

<main class="container">
  <!-- Navigation for test pages -->
  <nav data-testid="nav" class="test-nav">
    <a href="#main" data-testid="nav-main">Main</a>
    <a href="#forms" data-testid="nav-forms">Forms</a>
    <a href="#frames" data-testid="nav-frames">Frames</a>
    <a href="#shadow" data-testid="nav-shadow">Shadow</a>
    <a href="#alerts" data-testid="nav-alerts">Alerts</a>
    <a href="#scroll" data-testid="nav-scroll">Scroll</a>
  </nav>

  {#if currentRoute === '#frames'}
    <FrameTest />
  {:else if currentRoute === '#shadow'}
    <ShadowTest />
  {:else if currentRoute === '#alerts'}
    <AlertTest />
  {:else if currentRoute === '#scroll'}
    <ScrollTest />
  {:else if currentRoute === '#forms'}
    <FormTest />
  {:else}
    <!-- Main page content -->
    <h1 data-testid="welcome-heading">Welcome to Tauri!</h1>

    <div class="row" data-testid="logo-row">
      <a href="https://vite.dev" target="_blank" data-testid="link-vite">
        <img src="/vite.svg" class="logo vite" alt="Vite Logo" data-testid="logo-vite" />
      </a>
      <a href="https://tauri.app" target="_blank" data-testid="link-tauri">
        <img src="/tauri.svg" class="logo tauri" alt="Tauri Logo" data-testid="logo-tauri" />
      </a>
      <a href="https://svelte.dev" target="_blank" data-testid="link-svelte">
        <img src="/svelte.svg" class="logo svelte" alt="Svelte Logo" data-testid="logo-svelte" />
      </a>
    </div>

    <p data-testid="instruction-text">
      Click on the Tauri, Vite, and Svelte logos to learn more.
    </p>

    <div class="row" data-testid="greet-section">
      <Greet />
    </div>

    <!-- Additional test elements for locator testing -->
    <section data-testid="links-section" class="test-section">
      <h2 data-testid="links-heading">Link Text Test</h2>
      <a href="#link1" data-testid="exact-link">Click Here For Link One</a>
      <a href="#link2" data-testid="partial-link">Another Partial Link Example</a>
    </section>

    <section data-testid="visibility-section" class="test-section">
      <h2 data-testid="visibility-heading">Visibility Test</h2>
      <div data-testid="hidden-display" style="display: none;">Hidden by display</div>
      <div data-testid="hidden-visibility" style="visibility: hidden;">Hidden by visibility</div>
      <div data-testid="visible-element">Visible element</div>
      <button data-testid="disabled-button" disabled>Disabled Button</button>
      <button data-testid="enabled-button">Enabled Button</button>
    </section>

    <section data-testid="rect-section" class="test-section">
      <h2 data-testid="rect-heading">Element Rect Test</h2>
      <div class="rect-container">
        <div
          data-testid="positioned-element"
          style="position: absolute; left: 50px; top: 50px; width: 100px; height: 80px; background: #4a9eff;"
        >
          Positioned
        </div>
      </div>
    </section>
  {/if}
</main>

<style>
  .logo.vite:hover {
    filter: drop-shadow(0 0 2em #747bff);
  }

  .logo.svelte:hover {
    filter: drop-shadow(0 0 2em #ff3e00);
  }

  .test-nav {
    display: flex;
    gap: 1rem;
    margin-bottom: 1rem;
    padding: 0.5rem;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 4px;
  }

  .test-nav a {
    color: inherit;
    text-decoration: none;
    padding: 0.25rem 0.5rem;
  }

  .test-nav a:hover {
    text-decoration: underline;
  }

  .test-section {
    margin-top: 1rem;
    padding: 1rem;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
  }

  .test-section h2 {
    margin-top: 0;
    font-size: 1rem;
  }

  .rect-container {
    position: relative;
    width: 300px;
    height: 200px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px dashed rgba(255, 255, 255, 0.3);
  }
</style>
