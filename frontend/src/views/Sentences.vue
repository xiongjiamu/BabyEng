<template>
  <div class="page">
    <header class="appbar">
      <router-link class="icon-btn" to="/learn" aria-label="返回">←</router-link>
      <h1>情景短句</h1>
      <span class="icon-btn" aria-hidden="true"></span>
    </header>

    <div class="timetabs">
      <button
        v-for="t in scenes"
        :key="t.key"
        :aria-pressed="scene === t.key"
        @click="scene = t.key"
      >{{ t.icon }} {{ t.label }}</button>
    </div>

    <div class="page-body pad stack-5" style="padding-top:0">
      <div class="banner info">
        <span class="ico">🌅</span>
        <span>现在是{{ nowLabel }}，先用{{ currentLabel }}这几句。<br>同一句每天在同一个场景重复，比一次学很多句管用。</span>
      </div>

      <div v-for="s in filtered" :key="s.id" class="card sent-card">
        <div>
          <div class="en">{{ s.en }}</div>
          <div class="ipa">{{ s.phonetic || '' }}</div>
          <div class="zh">{{ s.zh }}</div>
        </div>
        <div v-if="s.example_context" class="ctx">{{ s.example_context }}</div>
        <div class="sent-acts">
          <button class="btn btn-primary" @click="playSentence(s)">▶ 播放</button>
          <router-link class="btn btn-ghost" :to="{ path: '/compare', query: { target_type: 'sentence', target_id: s.id, en: s.en, zh: s.zh, phonetic: s.phonetic, emoji: '💬' } }">🎤 跟读</router-link>
        </div>
      </div>

      <div v-if="filtered.length === 0" class="card center-text stack-3">
        <p class="t-mom">这个时段的句子还没准备好。</p>
      </div>

      <p class="note">
        PRD 8.2：句子的中文说法变体比词更多（「该睡觉了 / 要睡觉了 / 去睡觉」），因此句子同样带 aliases 字段，
        否则母亲在「问一问」里换个说法就查不到。
      </p>
    </div>

    <TabBar current="home" />
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue'
import TabBar from '../components/TabBar.vue'
import { useAppStore } from '../stores/app'
import { api } from '../api'
import { useAudio } from '../composables/useAudio'

const store = useAppStore()
const { playUrl, unlock } = useAudio()

const scenes = [
  { key: 'morning', label: '起床', icon: '🌅' },
  { key: 'meal', label: '吃饭', icon: '🍚' },
  { key: 'play', label: '玩耍', icon: '🧸' },
  { key: 'bedtime', label: '睡前', icon: '🌙' },
  { key: 'outing', label: '出门', icon: '🚪' },
]

const all = ref([])
const scene = ref(defaultScene())

function defaultScene() {
  const h = new Date().getHours()
  if (h < 9) return 'morning'
  if (h < 13) return 'meal'
  if (h < 18) return 'play'
  return 'bedtime'
}

const currentLabel = computed(() => scenes.find((s) => s.key === scene.value)?.label || '')
const nowLabel = computed(() => {
  const d = new Date()
  return `${d.getHours()} 点 ${String(d.getMinutes()).padStart(2, '0')} 分`
})
const filtered = computed(() => all.value.filter((s) => s.scene === scene.value))

onMounted(async () => {
  await store.bootstrap()
  unlock()
  try {
    const res = await api.sentences('')
    all.value = res.sentences || []
  } catch {
    all.value = []
  }
})

function playSentence(s) {
  playUrl(api.ttsUrl(s.en, store.settings.ttsRate, store.settings.ttsVoice), { rate: store.settings.ttsRate })
}
</script>
