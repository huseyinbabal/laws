<script setup lang="ts">
import ServiceBrowser from './ServiceBrowser.vue'
import ServiceGrid from './ServiceGrid.vue'
</script>

<template>
  <div class="space-y-6">
    <div class="text-center space-y-2">
      <h1 class="text-2xl font-bold tracking-tight">AWS Services</h1>
      <p class="text-sm text-muted-foreground">
        Browse and monitor local AWS service emulations
      </p>
    </div>

    <ServiceBrowser />

    <ServiceGrid />
  </div>
</template>
