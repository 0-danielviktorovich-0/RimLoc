<script lang="ts">
  // Project tab (workspace): renders the W5 authoritative project overview —
  // identity, source → target locales, locations, health, source-update state
  // and the danger zone live in ProjectOverview so the same view can be
  // reached from anywhere a project summary is needed. This wrapper keeps the
  // workspace seam (mandate §7: overview ≠ Settings clone).
  import ProjectOverview from '../screens/ProjectOverview.svelte';
</script>

<ProjectOverview />
