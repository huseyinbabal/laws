<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { Pin, PinOff } from 'lucide-vue-next'
import { AWS_SERVICES, SERVICE_CATEGORIES } from '../data/aws-services'
import type { AwsService } from '../data/aws-services'
import { usePinnedServices } from '../composables/usePinnedServices'

const router = useRouter()
const { togglePin, isPinned } = usePinnedServices()
const selectedCategory = ref<string | null>(null)

const displayed = computed(() => {
  if (!selectedCategory.value) return AWS_SERVICES
  return AWS_SERVICES.filter((s) => s.category === selectedCategory.value)
})

const grouped = computed(() => {
  const map = new Map<string, AwsService[]>()
  for (const s of displayed.value) {
    const list = map.get(s.category) || []
    list.push(s)
    map.set(s.category, list)
  }
  return [...map.entries()].sort(([a], [b]) => a.localeCompare(b))
})

function toggleCategory(cat: string) {
  selectedCategory.value = selectedCategory.value === cat ? null : cat
}

function goToService(serviceId: string) {
  router.push(`/services/${serviceId}`)
}
</script>

<template>
  <div class="space-y-6">
    <!-- Category filter pills -->
    <div class="flex flex-wrap gap-1.5">
      <button
        @click="selectedCategory = null"
        :class="[
          'px-2.5 py-1 rounded-md text-xs font-medium transition-colors',
          !selectedCategory
            ? 'bg-primary text-primary-foreground'
            : 'bg-secondary text-secondary-foreground hover:bg-secondary/80',
        ]"
      >
        All ({{ AWS_SERVICES.length }})
      </button>
      <button
        v-for="cat in SERVICE_CATEGORIES"
        :key="cat"
        @click="toggleCategory(cat)"
        :class="[
          'px-2.5 py-1 rounded-md text-xs font-medium transition-colors',
          selectedCategory === cat
            ? 'bg-primary text-primary-foreground'
            : 'bg-secondary text-secondary-foreground hover:bg-secondary/80',
        ]"
      >
        {{ cat }} ({{ AWS_SERVICES.filter((s) => s.category === cat).length }})
      </button>
    </div>

    <!-- Service cards -->
    <div v-for="[category, services] in grouped" :key="category">
      <h3 class="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-2">
        {{ category }}
      </h3>
      <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-2">
        <div
          v-for="service in services"
          :key="service.id"
          class="relative flex items-center gap-2.5 px-3 py-2.5 rounded-lg border border-border bg-card hover:bg-accent/40 hover:border-primary/30 transition-all group"
        >
          <button
            @click="goToService(service.id)"
            class="flex items-center gap-2.5 flex-1 min-w-0 text-left"
          >
            <img
              :src="service.iconUrl"
              alt=""
              class="w-7 h-7 flex-shrink-0 group-hover:scale-110 transition-transform"
              loading="lazy"
            />
            <div class="min-w-0">
              <div class="text-xs font-medium truncate">{{ service.name }}</div>
            </div>
          </button>
          <button
            @click.stop="togglePin(service.id)"
            :title="isPinned(service.id) ? 'Unpin from navbar' : 'Pin to navbar'"
            :class="[
              'p-0.5 rounded transition-colors flex-shrink-0',
              isPinned(service.id)
                ? 'text-primary opacity-100'
                : 'text-muted-foreground/40 opacity-0 group-hover:opacity-100 hover:text-foreground',
            ]"
          >
            <PinOff v-if="isPinned(service.id)" class="h-3 w-3" />
            <Pin v-else class="h-3 w-3" />
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
