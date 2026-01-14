<script lang="ts">
  import '../styles/global.css';

  interface Props {
    children: import('svelte').Snippet;
  }

  let { children }: Props = $props();
</script>

{@render children()}

<style>
  :global(#app) {
    height: 100vh;
    width: 100vw;
  }
</style>
