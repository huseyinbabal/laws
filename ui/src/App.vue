<script setup lang="ts">
import { provideSSE } from './composables/useSSE'
import { providePinnedServices } from './composables/usePinnedServices'

provideSSE()
providePinnedServices()
</script>

<template>
  <router-view />
</template>
