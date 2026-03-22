<script setup lang="ts">
import { computed } from 'vue'
import { cn } from '../../lib/utils'
import { cva } from 'class-variance-authority'

const props = defineProps<{
  variant?: 'default' | 'secondary' | 'destructive' | 'outline' | 'success' | 'warning' | 'info'
  class?: string
}>()

const badgeVariants = cva(
  'inline-flex items-center rounded-md px-2 py-0.5 text-xs font-medium transition-colors',
  {
    variants: {
      variant: {
        default: 'bg-primary/20 text-primary',
        secondary: 'bg-secondary text-secondary-foreground',
        destructive: 'bg-destructive/20 text-destructive',
        outline: 'border text-foreground',
        success: 'bg-emerald-500/20 text-emerald-400',
        warning: 'bg-amber-500/20 text-amber-400',
        info: 'bg-sky-500/20 text-sky-400',
      },
    },
    defaultVariants: {
      variant: 'default',
    },
  },
)

const classes = computed(() => cn(badgeVariants({ variant: props.variant || 'default' }), props.class))
</script>

<template>
  <div :class="classes">
    <slot />
  </div>
</template>
