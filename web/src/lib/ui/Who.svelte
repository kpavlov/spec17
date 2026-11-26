<script lang="ts">
  // (WEB-36) who last changed a line: `note` is the sentence under a heading, else the short form beside a reference
  let { commit: c, note = false }: { commit: any; note?: boolean } = $props();
</script>

{#if c && note}<p class="muted note">Last changed by <b>{c.author}</b>, {c.date} · <span class="mono">{c.short}</span> {c.subject}</p>
{:else if c}<span class="muted who" title={c.subject}>{c.author}, {c.date} · <span class="mono">{c.short}</span></span>{/if}
