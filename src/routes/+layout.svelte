<script lang="ts">
  // CSS Import Order:
  // 1. Design tokens (Style Dictionary generated + Open Props)
  // 2. Theme definitions (global.css)
  // 3. Component styles
  import '../styles/generated/tokens.css';
  import '../styles/open-props.css';
  import '../styles/global.css';
  import '../styles/components.css';

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
