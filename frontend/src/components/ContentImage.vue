<template>
  <span class="content-image">
    <img v-if="!failed && targetId" :src="src" :alt="alt" @load="loaded" @error="fallback" />
    <span v-else class="emoji" aria-hidden="true">{{ emoji || '🖼' }}</span>
  </span>
</template>

<script setup>
import { computed, ref, watch } from 'vue'
import { api } from '../api'

const props = defineProps({
  kind: { type: String, required: true },
  targetId: { type: String, default: '' },
  emoji: { type: String, default: '🖼' },
  alt: { type: String, default: '' },
  version: { type: [String, Number], default: '' },
})
const emit = defineEmits(['loaded', 'fallback'])
const failed = ref(false)
const src = computed(() => api.contentImageUrl(props.kind, props.targetId, props.version))
watch(src, () => { failed.value = false })
function loaded() { emit('loaded') }
function fallback() { failed.value = true; emit('fallback') }
</script>

<style scoped>
.content-image { display:inline-flex;align-items:center;justify-content:center;overflow:hidden; }
.content-image img { display:block;width:100%;height:100%;object-fit:cover; }
</style>
