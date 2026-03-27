<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, RouterLink } from 'vue-router'
import { ArrowLeft, ExternalLink, Activity, Clock, AlertCircle, Pin, PinOff } from 'lucide-vue-next'
import { SERVICE_BY_ID } from '../data/aws-services'
import { useSSE } from '../composables/useSSE'
import { usePinnedServices } from '../composables/usePinnedServices'
import AppBadge from './ui/AppBadge.vue'
import AppCard from './ui/AppCard.vue'
import RequestTable from './RequestTable.vue'
import ResourcePanel from './ResourcePanel.vue'

const route = useRoute()
const { events } = useSSE()
const { togglePin, isPinned } = usePinnedServices()

const serviceId = computed(() => route.params.serviceId as string)
const service = computed(() => (serviceId.value ? SERVICE_BY_ID.get(serviceId.value) : undefined))

const serviceEvents = computed(() => {
  if (!service.value) return []
  return events.value.filter((e) => e.service === service.value!.name)
})

const stats = computed(() => {
  const evts = serviceEvents.value
  const total = evts.length
  const errors = evts.filter((e) => e.status_code >= 400).length
  const avgDuration =
    total > 0 ? Math.round(evts.reduce((sum, e) => sum + e.duration_ms, 0) / total) : 0
  const actions = [...new Set(evts.map((e) => e.action))].sort()
  return { total, errors, avgDuration, actions }
})
</script>

<template>
  <!-- Service not found -->
  <div v-if="!service" class="flex flex-col items-center justify-center h-[60vh] gap-4">
    <AlertCircle class="h-12 w-12 text-muted-foreground" />
    <h2 class="text-lg font-medium">Service not found</h2>
    <RouterLink
      to="/"
      class="text-sm text-primary hover:underline flex items-center gap-1"
    >
      <ArrowLeft class="h-3.5 w-3.5" />
      Back to services
    </RouterLink>
  </div>

  <!-- Service detail -->
  <div v-else class="space-y-6">
    <!-- Header -->
    <div class="flex items-start gap-4">
      <RouterLink
        to="/"
        class="mt-1 p-1.5 rounded-md hover:bg-accent/50 transition-colors text-muted-foreground hover:text-foreground"
      >
        <ArrowLeft class="h-4 w-4" />
      </RouterLink>
      <img :src="service.iconUrl" :alt="service.name" class="w-12 h-12 flex-shrink-0" />
      <div class="flex-1 min-w-0">
        <div class="flex items-center gap-3">
          <h1 class="text-xl font-semibold">{{ service.name }}</h1>
          <AppBadge variant="outline" class="text-[10px]">{{ service.category }}</AppBadge>
          <button
            @click="togglePin(service.id)"
            :title="isPinned(service.id) ? 'Unpin from navbar' : 'Pin to navbar'"
            :class="[
              'p-1 rounded-md transition-colors',
              isPinned(service.id)
                ? 'text-primary hover:bg-accent/50'
                : 'text-muted-foreground hover:text-foreground hover:bg-accent/50',
            ]"
          >
            <PinOff v-if="isPinned(service.id)" class="h-4 w-4" />
            <Pin v-else class="h-4 w-4" />
          </button>
        </div>
        <p class="text-sm text-muted-foreground mt-0.5">{{ service.description }}</p>
      </div>
    </div>

    <!-- Stats -->
    <div class="grid grid-cols-3 gap-3">
      <AppCard>
        <template #header>
          <Activity class="h-4 w-4" />
          Requests
        </template>
        <p class="text-2xl font-bold tabular-nums">{{ stats.total }}</p>
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
          <AlertCircle class="h-4 w-4" />
          Errors
        </template>
        <p class="text-2xl font-bold tabular-nums text-destructive">{{ stats.errors }}</p>
      </AppCard>
    </div>

    <!-- Resources -->
    <ResourcePanel :serviceId="serviceId" />

    <!-- Actions seen -->
    <div v-if="stats.actions.length > 0">
      <h2 class="text-sm font-medium text-muted-foreground mb-2">Actions Seen</h2>
      <div class="flex flex-wrap gap-1.5">
        <AppBadge v-for="action in stats.actions" :key="action" variant="secondary" class="text-xs">
          {{ action }}
        </AppBadge>
      </div>
    </div>

    <!-- Recent requests -->
    <div>
      <div class="flex items-center justify-between mb-2">
        <h2 class="text-sm font-medium text-muted-foreground">Recent Requests</h2>
        <RouterLink to="/logs" class="text-xs text-primary hover:underline flex items-center gap-1">
          View all logs
          <ExternalLink class="h-3 w-3" />
        </RouterLink>
      </div>

      <div
        v-if="serviceEvents.length === 0"
        class="flex items-center justify-center h-32 rounded-lg border border-border text-sm text-muted-foreground"
      >
        No requests recorded for this service yet
      </div>

      <RequestTable v-else :events="serviceEvents.slice(0, 50)" />
    </div>
  </div>
</template>
