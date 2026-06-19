<script>
  import { Router } from 'svelte-routing';
  import App from '../app/App.svelte';
  export let url = '/';
</script>

<Router {url}>
  <App />
</Router>
