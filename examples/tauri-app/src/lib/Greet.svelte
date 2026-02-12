<script>
  import { invoke } from "@tauri-apps/api/core"

  let name = $state("");
  let greetMsg = $state("")

  async function greet(){
    greetMsg = await invoke("greet", { name })
  }
</script>

<div data-testid="greet-container">
  <div class="row">
    <input
      id="greet-input"
      data-testid="greet-input"
      placeholder="Enter a name..."
      bind:value={name}
    />
    <button data-testid="greet-button" onclick={greet}>
      Greet
    </button>
  </div>
  <p data-testid="greet-message">{greetMsg}</p>
</div>
