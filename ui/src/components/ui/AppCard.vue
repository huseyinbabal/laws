<script setup lang="ts">
defineProps<{
  class?: string
}>()
</script>

<template>
  <div :class="['rounded-xl border bg-card text-card-foreground shadow', $props.class]">
    <div class="flex flex-col space-y-1.5 p-4">
      <h3 class="font-medium text-sm text-muted-foreground flex items-center gap-2">
        <slot name="header" />
      </h3>
    </div>
    <div class="p-4 pt-0">
      <slot />
    </div>
  </div>
</template>
