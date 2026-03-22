<script setup lang="ts">
import { computed } from 'vue'
import { RouterLink, RouterView, useRoute } from 'vue-router'
import { Radio, ScrollText, Pin } from 'lucide-vue-next'
import { useSSE } from '../composables/useSSE'
import { usePinnedServices } from '../composables/usePinnedServices'

const { connected } = useSSE()
const { pinnedServices } = usePinnedServices()
const route = useRoute()

const isServicesActive = computed(() => route.path === '/' || route.path === '')
const isLogsActive = computed(() => route.path === '/logs')

const pinned = computed(() => pinnedServices())

function navClass(active: boolean): string {
  return active
    ? 'px-2.5 py-1 rounded-md text-xs font-medium transition-colors bg-accent text-foreground'
    : 'px-2.5 py-1 rounded-md text-xs font-medium transition-colors text-muted-foreground hover:text-foreground hover:bg-accent/50'
}

function isPinnedActive(serviceId: string): boolean {
  return route.path === `/services/${serviceId}`
}
</script>

<template>
  <div class="min-h-screen bg-background text-foreground">
    <!-- Top navbar -->
    <nav class="sticky top-0 z-40 h-12 border-b border-border bg-card/80 backdrop-blur-sm">
      <div class="h-full max-w-[1400px] mx-auto px-4 flex items-center justify-between">
        <!-- Left: logo + nav links + pinned services -->
        <div class="flex items-center gap-6 min-w-0">
          <RouterLink to="/" class="flex items-center gap-2 flex-shrink-0">
            <div class="w-6 h-6 rounded bg-primary flex items-center justify-center">
              <span class="text-[10px] font-bold text-primary-foreground">L</span>
            </div>
            <span class="text-sm font-semibold tracking-tight">laws</span>
          </RouterLink>

          <div class="flex items-center gap-1">
            <RouterLink to="/" :class="navClass(isServicesActive)">
              Services
            </RouterLink>
          </div>

          <!-- Pinned services -->
          <div v-if="pinned.length > 0" class="flex items-center gap-1 pl-3 border-l border-border overflow-x-auto">
            <Pin class="h-3 w-3 text-muted-foreground/50 flex-shrink-0 rotate-45" />
            <RouterLink
              v-for="svc in pinned"
              :key="svc.id"
              :to="`/services/${svc.id}`"
              :class="[
                'flex items-center gap-1.5 flex-shrink-0',
                navClass(isPinnedActive(svc.id)),
              ]"
              :title="svc.name"
            >
              <img :src="svc.iconUrl" :alt="svc.name" class="w-4 h-4" />
              <span class="max-w-[80px] truncate">{{ svc.name }}</span>
            </RouterLink>
          </div>
        </div>

        <!-- Right: live logs link + connection status -->
        <div class="flex items-center gap-3 flex-shrink-0">
          <RouterLink
            to="/logs"
            :class="[
              'flex items-center gap-1.5',
              navClass(isLogsActive),
            ]"
          >
            <ScrollText class="h-3.5 w-3.5" />
            Live Logs
          </RouterLink>

          <div class="flex items-center gap-1.5 pl-3 border-l border-border">
            <Radio
              :class="[
                'h-3.5 w-3.5',
                connected ? 'text-emerald-400 animate-pulse' : 'text-destructive',
              ]"
            />
            <span class="text-[10px] text-muted-foreground">
              {{ connected ? 'Connected' : 'Disconnected' }}
            </span>
          </div>
        </div>
      </div>
    </nav>

    <!-- Main content -->
    <main class="max-w-[1400px] mx-auto px-4 py-6">
      <RouterView />
    </main>
  </div>
</template>
