<script>
  let alertResult = $state('');
  let confirmResult = $state('');
  let promptResult = $state('');

  function showAlert() {
    alert('This is a test alert message!');
    alertResult = 'Alert was shown and dismissed';
  }

  function showConfirm() {
    const result = confirm('Do you want to confirm this action?');
    confirmResult = result ? 'User clicked OK' : 'User clicked Cancel';
  }

  function showPrompt() {
    const result = prompt('Please enter your name:', 'Default Value');
    if (result === null) {
      promptResult = 'User cancelled the prompt';
    } else {
      promptResult = `User entered: ${result}`;
    }
  }

  function showCustomAlert() {
    alert('Custom message: Hello from WebDriver test!');
    alertResult = 'Custom alert was shown';
  }
</script>

<section data-testid="alert-test-page">
  <h1 data-testid="alert-page-heading">Alert Test Page</h1>

  <p data-testid="alert-page-description">This page tests alert, confirm, and prompt dialogs.</p>

  <div class="alert-section">
    <h2>Alert Dialog</h2>
    <button data-testid="alert-button" onclick={showAlert}>Show Alert</button>
    <button data-testid="custom-alert-button" onclick={showCustomAlert}>Show Custom Alert</button>
    <p data-testid="alert-result">{alertResult}</p>
  </div>

  <div class="alert-section">
    <h2>Confirm Dialog</h2>
    <button data-testid="confirm-button" onclick={showConfirm}>Show Confirm</button>
    <p data-testid="confirm-result">{confirmResult}</p>
  </div>

  <div class="alert-section">
    <h2>Prompt Dialog</h2>
    <button data-testid="prompt-button" onclick={showPrompt}>Show Prompt</button>
    <p data-testid="prompt-result">{promptResult}</p>
  </div>

  <div class="alert-section">
    <h2>Delayed Alert</h2>
    <button data-testid="delayed-alert-button" onclick={() => setTimeout(() => alert('Delayed alert!'), 1000)}>
      Show Delayed Alert (1s)
    </button>
  </div>
</section>

<style>
  .alert-section {
    margin: 1rem 0;
    padding: 1rem;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
  }

  .alert-section h2 {
    margin-top: 0;
    font-size: 1rem;
  }

  button {
    margin-right: 0.5rem;
    margin-bottom: 0.5rem;
  }

  p {
    margin-top: 0.5rem;
    min-height: 1.5em;
  }
</style>
