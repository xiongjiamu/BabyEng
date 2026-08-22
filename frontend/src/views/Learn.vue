<template>
  <div class="page">
    <header class="appbar">
      <router-link class="icon-btn" to="/home" aria-label="返回">←</router-link>
      <h1>学一学</h1>
      <span class="icon-btn" aria-hidden="true"></span>
    </header>

    <div class="page-body pad stack-5">
      <p class="t-mom" style="margin:0">以下都是<b>有编排</b>的学习流，内容顺序由应用决定。每次选一个主题，轻松学一小组。</p>

      <div class="duo">
        <router-link class="tile" to="/subject-learn/chinese">
          <span class="ico">📖</span><span class="t">语文启蒙</span><span class="s">常用字、反义词、儿歌</span>
        </router-link>
        <router-link class="tile" to="/subject-learn/math">
          <span class="ico">🔢</span><span class="t">数学启蒙</span><span class="s">数数、多少、形状</span>
        </router-link>
      </div>

      <router-link class="mode s1" to="/sentences">
        <span class="ico">💬</span>
        <span class="grow">
          <span class="t">情景短句</span>
          <span class="s">按生活时段浏览短句卡，逐句听 + 跟读</span>
          <span class="m">{{ currentSceneLabel }} · 建议「{{ currentSceneLabel }}」组 {{ sceneCount }} 句</span>
        </span>
      </router-link>

      <router-link class="mode s2" to="/word-learn">
        <span class="ico">📚</span>
        <span class="grow">
          <span class="t">单词学习</span>
          <span class="s">按场景分类，看图 — 听音 — 跟读，每 5 词一组</span>
          <span class="m">物品 {{ learnedStats.item || 0 }}/{{ totalStats.item || 30 }} · 人物 {{ learnedStats.person || 0 }}/{{ totalStats.person || 8 }}</span>
        </span>
      </router-link>

      <router-link class="mode s3" to="/review">
        <span class="ico">🔁</span>
        <span class="grow">
          <span class="t">单词复习</span>
          <span class="s">只推送学过、但还没稳的词</span>
          <span class="m">今天 {{ reviewCount }} 个待复习</span>
        </span>
      </router-link>

      <div class="divider"></div>

      <p class="note">语音问答支持自由提问；学习模式有固定编排，内容顺序由应用决定。</p>
    </div>

    <TabBar current="home" />
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue'
import TabBar from '../components/TabBar.vue'
import { useAppStore } from '../stores/app'
import { api } from '../api'

const store = useAppStore()
const reviewCount = ref(0)
const learnedStats = ref({})
const totalStats = ref({})

const nowHour = new Date().getHours()
const currentScene = nowHour < 8 ? 'morning' : nowHour < 11 ? 'morning' : nowHour < 13 ? 'meal' : nowHour < 18 ? 'play' : nowHour < 21 ? 'bedtime' : 'bedtime'
const currentSceneLabel = computed(() => ({ morning: '起床', meal: '吃饭', play: '玩耍', bedtime: '睡前', outing: '出门' })[currentScene] || '起床')
const sceneCount = computed(() => 4)

onMounted(async () => {
  await store.bootstrap()
  if (!store.initialized) { location.href = '/onboarding'; return }
  try {
    const cid = store.childId
    const [scenes, queue] = await Promise.all([api.scenes(cid), api.reviewQueue(cid)])
    const s = scenes.scenes || []
    const map = {}
    const total = {}
    s.forEach((x) => {
      const cat = x.category.startsWith('item') ? 'item' : x.category.startsWith('person') ? 'person' : x.category
      total[cat] = (total[cat] || 0) + x.total
      map[cat] = (map[cat] || 0) + x.learned
    })
    learnedStats.value = map
    totalStats.value = total
    reviewCount.value = queue.count
  } catch { /* ignore */ }
})
</script>
