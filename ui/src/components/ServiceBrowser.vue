<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { Search } from 'lucide-vue-next'
import { AWS_SERVICES, type AwsService } from '../data/aws-services'

const router = useRouter()
const query = ref('')
const isOpen = ref(false)
const selectedIndex = ref(0)
const inputRef = ref<HTMLInputElement | null>(null)
const listRef = ref<HTMLDivElement | null>(null)

const filtered = computed(() => {
  if (!query.value.trim()) return AWS_SERVICES
  const q = query.value.toLowerCase()
  return AWS_SERVICES.filter(
    (s) =>
      s.name.toLowerCase().includes(q) ||
      s.id.includes(q) ||
      s.category.toLowerCase().includes(q) ||
      s.description.toLowerCase().includes(q),
  )
})

const grouped = computed(() => {
  const map = new Map<string, AwsService[]>()
  for (const s of filtered.value) {
    const list = map.get(s.category) || []
    list.push(s)
    map.set(s.category, list)
  }
  return [...map.entries()].sort(([a], [b]) => a.localeCompare(b))
})

const flatList = computed(() => grouped.value.flatMap(([, services]) => services))

watch(query, () => {
  selectedIndex.value = 0
})

watch(selectedIndex, async () => {
  await nextTick()
  if (!listRef.value || flatList.value.length === 0) return
  const item = listRef.value.querySelector(`[data-index="${selectedIndex.value}"]`)
  item?.scrollIntoView({ block: 'nearest' })
})

function handleSelect(service: AwsService) {
  query.value = ''
  isOpen.value = false
  router.push(`/services/${service.id}`)
}

function handleKeyDown(e: KeyboardEvent) {
  if (!isOpen.value && e.key !== 'Escape') {
    isOpen.value = true
  }

  switch (e.key) {
    case 'ArrowDown':
      e.preventDefault()
      selectedIndex.value = Math.min(selectedIndex.value + 1, flatList.value.length - 1)
      break
    case 'ArrowUp':
      e.preventDefault()
      selectedIndex.value = Math.max(selectedIndex.value - 1, 0)
      break
    case 'Enter':
      e.preventDefault()
      if (flatList.value[selectedIndex.value]) {
        handleSelect(flatList.value[selectedIndex.value])
      }
      break
    case 'Escape':
      e.preventDefault()
      isOpen.value = false
      inputRef.value?.blur()
      break
  }
}

function onInput(e: Event) {
  query.value = (e.target as HTMLInputElement).value
  isOpen.value = true
}

function onFocus() {
  isOpen.value = true
}

function onBlur() {
  setTimeout(() => {
    isOpen.value = false
  }, 200)
}

function handleGlobalKeyDown(e: KeyboardEvent) {
  if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
    e.preventDefault()
    inputRef.value?.focus()
  }
}

onMounted(() => {
  document.addEventListener('keydown', handleGlobalKeyDown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleGlobalKeyDown)
})
</script>

<template>
  <div class="relative w-full max-w-2xl mx-auto">
    <!-- Search input -->
    <div class="relative">
      <Search class="absolute left-3.5 top-1/2 -translate-y-1/2 h-4.5 w-4.5 text-muted-foreground" />
      <input
        ref="inputRef"
        type="text"
        :value="query"
        @input="onInput"
        @focus="onFocus"
        @blur="onBlur"
        @keydown="handleKeyDown"
        placeholder="Search AWS services..."
        class="w-full h-11 pl-10 pr-16 rounded-lg border border-border bg-card text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-primary/50 focus:border-primary transition-colors"
      />
      <kbd class="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none hidden sm:inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded border border-border bg-muted text-[10px] font-mono text-muted-foreground">
        <span class="text-xs">&#8984;</span>K
      </kbd>
    </div>

    <!-- Dropdown -->
    <div
      v-if="isOpen"
      ref="listRef"
      class="absolute top-full mt-1.5 left-0 right-0 z-50 bg-card border border-border rounded-lg shadow-xl max-h-[420px] overflow-y-auto"
    >
      <div v-if="flatList.length === 0" class="px-4 py-8 text-center text-sm text-muted-foreground">
        No services match "{{ query }}"
      </div>
      <template v-else>
        <div v-for="[category, services] in grouped" :key="category">
          <div class="sticky top-0 bg-card/95 backdrop-blur-sm px-3 py-1.5 text-[10px] font-semibold text-muted-foreground uppercase tracking-wider border-b border-border/50">
            {{ category }}
          </div>
          <button
            v-for="service in services"
            :key="service.id"
            :data-index="flatList.indexOf(service)"
            :class="[
              'w-full flex items-center gap-3 px-3 py-2 text-left transition-colors',
              flatList.indexOf(service) === selectedIndex ? 'bg-accent/60' : 'hover:bg-accent/30',
            ]"
            @mouseenter="selectedIndex = flatList.indexOf(service)"
            @mousedown="handleSelect(service)"
          >
            <img
              :src="service.iconUrl"
              alt=""
              class="w-6 h-6 flex-shrink-0"
              loading="lazy"
            />
            <div class="min-w-0 flex-1">
              <div class="text-sm font-medium truncate">{{ service.name }}</div>
              <div class="text-[11px] text-muted-foreground truncate">{{ service.description }}</div>
            </div>
          </button>
        </div>
      </template>
    </div>
  </div>
</template>
