<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  export let to = '#';
  export let replace = false;
  export let state = {};
  $: void replace;
  $: void state;
  const dispatch = createEventDispatcher();
  function handleClick(e: MouseEvent) {
    dispatch('click', e);
  }
</script>

<a href={to} on:click={handleClick}>
  <slot />
</a>
