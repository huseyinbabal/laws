<script setup lang="ts">
import { computed } from 'vue'
import { Activity, Zap, Layers, Clock } from 'lucide-vue-next'
import AppCard from './ui/AppCard.vue'
import type { ApiRequestEvent } from '../types/request'

const props = defineProps<{
  events: readonly ApiRequestEvent[]
}>()

const stats = computed(() => {
  const total = props.events.length
  const services = new Set(props.events.map((e) => e.service)).size
  const avgDuration =
    total > 0 ? Math.round(props.events.reduce((sum, e) => sum + e.duration_ms, 0) / total) : 0
  const errors = props.events.filter((e) => e.status_code >= 400).length
  return { total, services, avgDuration, errors }
})
</script>

<template>
  <div class="grid grid-cols-2 lg:grid-cols-4 gap-3">
    <AppCard>
      <template #header>
        <Activity class="h-4 w-4" />
        Total Requests
      </template>
      <p class="text-2xl font-bold tabular-nums">{{ stats.total }}</p>
    </AppCard>

    <AppCard>
      <template #header>
        <Layers class="h-4 w-4" />
        Services Hit
      </template>
      <p class="text-2xl font-bold tabular-nums">{{ stats.services }}</p>
    </AppCard>

    <AppCard>
      <template #header>
        <Clock class="h-4 w-4" />
        Avg Duration
      </template>
      <p class="text-2xl font-bold tabular-nums">{{ stats.avgDuration }}ms</p>
    </AppCard>

    <AppCard>
      <template #header>
        <Zap class="h-4 w-4" />
        Errors
      </template>
      <p class="text-2xl font-bold tabular-nums text-destructive">{{ stats.errors }}</p>
    </AppCard>
  </div>
</template>
